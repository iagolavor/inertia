//! Tauri shell: spawn `inertia-api` sidecar, wait for health, open the local UI.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, RunEvent};

const API_ORIGIN: &str = "http://127.0.0.1:4783";
const HEALTH_URL: &str = "http://127.0.0.1:4783/api/health";
const HEALTH_TIMEOUT: Duration = Duration::from_secs(45);
const HEALTH_POLL: Duration = Duration::from_millis(200);

struct ApiChild(Mutex<Option<Child>>);

impl ApiChild {
    fn kill(&self) {
        if let Ok(mut guard) = self.0.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Drop for ApiChild {
    fn drop(&mut self) {
        self.kill();
    }
}

fn web_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let resource = app
        .path()
        .resource_dir()
        .map_err(|e| format!("resource dir: {e}"))?;
    let bundled = resource.join("resources").join("web");
    if bundled.is_dir() && bundled.join("index.html").is_file() {
        return Ok(bundled);
    }
    // Packaged layouts sometimes flatten resources/
    let flat = resource.join("web");
    if flat.is_dir() && flat.join("index.html").is_file() {
        return Ok(flat);
    }
    // Dev: repo apps/web/build after `npm run web:build`
    let manifest_web = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/build");
    if manifest_web.is_dir() && manifest_web.join("index.html").is_file() {
        return Ok(manifest_web.canonicalize().unwrap_or(manifest_web));
    }
    Err(format!(
        "UI assets not found (tried {}, {}, {})",
        bundled.display(),
        flat.display(),
        manifest_web.display()
    ))
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create data dir: {e}"))?;
    Ok(dir)
}

fn sidecar_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current exe: {e}"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "current exe has no parent".to_string())?;
    // tauri-build copies externalBin next to the app as the basename (triple stripped).
    #[cfg(windows)]
    let path = dir.join("inertia-api.exe");
    #[cfg(not(windows))]
    let path = dir.join("inertia-api");
    if !path.is_file() {
        return Err(format!("sidecar missing at {}", path.display()));
    }
    Ok(path)
}

fn health_ok() -> bool {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()
        .and_then(|c| c.get(HEALTH_URL).send().ok())
        .is_some_and(|resp| resp.status().is_success())
}

fn wait_for_health(child: &mut Child) -> Result<(), String> {
    let deadline = Instant::now() + HEALTH_TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("inertia-api exited before becoming healthy ({status})"));
        }
        if health_ok() {
            return Ok(());
        }
        thread::sleep(HEALTH_POLL);
    }
    Err(format!(
        "inertia-api did not become healthy at {HEALTH_URL} within {}s",
        HEALTH_TIMEOUT.as_secs()
    ))
}

fn start_api(app: &AppHandle) -> Result<Child, String> {
    if health_ok() {
        return Err(
            "port 4783 already in use (another inertia-api is running). Stop it first.".into(),
        );
    }

    let data = data_dir(app)?;
    let web = web_dir(app)?;
    let bin = sidecar_path()?;

    eprintln!(
        "inertia-desktop: starting sidecar data={} web={}",
        data.display(),
        web.display()
    );

    let mut command = Command::new(&bin);
    command
        .env("INERTIA_DATA_DIR", &data)
        .env("INERTIA_WEB_DIR", &web)
        .env("INERTIA_API_ADDR", "127.0.0.1:4783")
        .env("RUST_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    // If the shell dies abruptly, ask the kernel to terminate the sidecar too.
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
    }

    let mut child = command
        .spawn()
        .map_err(|e| format!("spawn {}: {e}", bin.display()))?;

    if let Err(e) = wait_for_health(&mut child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e);
    }
    Ok(child)
}

fn open_ui(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window missing".to_string())?;
    // Load API-hosted SPA (same origin as Capacitor / Windows zip).
    window
        .eval(&format!("window.location.replace('{API_ORIGIN}/')"))
        .map_err(|e| format!("navigate to UI: {e}"))?;
    Ok(())
}

fn stop_api(app: &AppHandle) {
    if let Some(state) = app.try_state::<ApiChild>() {
        state.kill();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            match start_api(&handle) {
                Ok(child) => {
                    app.manage(ApiChild(Mutex::new(Some(child))));
                    if let Err(e) = open_ui(&handle) {
                        eprintln!("inertia-desktop: {e}");
                        stop_api(&handle);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("inertia-desktop: {e}");
                    std::process::exit(1);
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building inertia-desktop")
        .run(|app_handle, event| {
            if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
                stop_api(app_handle);
            }
        });
}
