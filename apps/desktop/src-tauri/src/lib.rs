//! Tauri shell: desktop spawns `inertia-api` sidecar; Android uses jniLibs process (Kotlin).
//! Both wait for health then open the local UI at http://127.0.0.1:4783.

use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
#[cfg(desktop)]
use tauri::RunEvent;

const API_ORIGIN: &str = "http://127.0.0.1:4783";
const HEALTH_URL: &str = "http://127.0.0.1:4783/api/health";
const HEALTH_TIMEOUT: Duration = Duration::from_secs(45);
const HEALTH_POLL: Duration = Duration::from_millis(200);

fn health_ok() -> bool {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()
        .and_then(|c| c.get(HEALTH_URL).send().ok())
        .is_some_and(|resp| resp.status().is_success())
}

fn wait_for_health_ready() -> Result<(), String> {
    let deadline = Instant::now() + HEALTH_TIMEOUT;
    while Instant::now() < deadline {
        if health_ok() {
            return Ok(());
        }
        std::thread::sleep(HEALTH_POLL);
    }
    Err(format!(
        "inertia-api did not become healthy at {HEALTH_URL} within {}s",
        HEALTH_TIMEOUT.as_secs()
    ))
}

fn open_ui(app: &AppHandle, start_url: &str) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window missing".to_string())?;
    // Prefer native navigate over eval(location.replace): Android WebView often
    // ignores JS redirects from the bundled splash asset origin.
    let url = tauri::Url::parse(start_url).map_err(|e| format!("invalid start url: {e}"))?;
    window
        .navigate(url)
        .map_err(|e| format!("navigate to UI: {e}"))?;
    Ok(())
}

/// Optional invite URL written by SplashActivity before MainActivity starts.
#[cfg(mobile)]
fn pending_invite_url(app: &AppHandle) -> Option<String> {
    let dir = app.path().app_data_dir().ok()?;
    let path = dir.join("pending-invite-url");
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(mobile)]
fn start_url(app: &AppHandle) -> String {
    pending_invite_url(app).unwrap_or_else(|| format!("{API_ORIGIN}/"))
}

#[cfg(desktop)]
mod desktop {
    use super::*;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::sync::Mutex;
    use std::thread;

    pub struct ApiChild(pub Mutex<Option<Child>>);

    impl ApiChild {
        pub fn kill(&self) {
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
        let flat = resource.join("web");
        if flat.is_dir() && flat.join("index.html").is_file() {
            return Ok(flat);
        }
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
        #[cfg(windows)]
        let path = dir.join("inertia-api.exe");
        #[cfg(not(windows))]
        let path = dir.join("inertia-api");
        if !path.is_file() {
            return Err(format!("sidecar missing at {}", path.display()));
        }
        Ok(path)
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

    pub fn start_api(app: &AppHandle) -> Result<Child, String> {
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

    pub fn stop_api(app: &AppHandle) {
        if let Some(state) = app.try_state::<ApiChild>() {
            state.kill();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();

            #[cfg(desktop)]
            {
                match desktop::start_api(&handle) {
                    Ok(child) => {
                        app.manage(desktop::ApiChild(std::sync::Mutex::new(Some(child))));
                        if let Err(e) = open_ui(&handle, &format!("{API_ORIGIN}/")) {
                            eprintln!("inertia-desktop: {e}");
                            desktop::stop_api(&handle);
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("inertia-desktop: {e}");
                        std::process::exit(1);
                    }
                }
            }

            #[cfg(mobile)]
            {
                // SplashActivity + InertiaApiService already started libinertia_api.so.
                if let Err(e) = wait_for_health_ready() {
                    eprintln!("inertia-desktop: {e}");
                    std::process::exit(1);
                }
                let url = start_url(&handle);
                if let Err(e) = open_ui(&handle, &url) {
                    eprintln!("inertia-desktop: {e}");
                    std::process::exit(1);
                }
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building inertia-desktop")
        .run(|app_handle, event| {
            #[cfg(desktop)]
            if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
                desktop::stop_api(app_handle);
            }
            #[cfg(mobile)]
            {
                let _ = (app_handle, event);
            }
        });
}
