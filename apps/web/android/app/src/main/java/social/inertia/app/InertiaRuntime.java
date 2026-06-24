package social.inertia.app;

import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Build;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.Map;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

/** Extracts and runs bundled inertia-api (Stage B — mirrors Windows zip layout). */
public final class InertiaRuntime {
    private static final String ASSET_WEB_DIR = "inertia/web";
    private static final String ASSET_BUNDLE_ID = "inertia/web/bundle-id";
    private static final String EXTRACTED_BUNDLE_ID = "bundle-id";
    /** Packaged in jniLibs; .so suffix so Android extracts it with execute permission. */
    private static final String NATIVE_LIB_NAME = "libinertia_api.so";
    private static final String API_HOST = "127.0.0.1";
    private static final int API_PORT = 4783;
    private static final String API_ADDR = API_HOST + ":" + API_PORT;
    private static final String HEALTH_URL = "http://" + API_ADDR + "/api/health";
    private static final String UI_URL = "http://" + API_ADDR + "/";

    private static Process process;

    private InertiaRuntime() {}

    public static boolean hasBundledApi(Context context) {
        if (resolveInstalledApiBinary(context) != null) {
            return true;
        }
        return apkContainsBundledApi(context);
    }

    public static String getUiUrl() {
        return UI_URL;
    }

    /** Stage B API origin — keep in WebView; Capacitor otherwise opens Chrome. */
    public static boolean isInertiaApiUrl(Uri uri) {
        return uri != null
            && "http".equals(uri.getScheme())
            && API_HOST.equals(uri.getHost())
            && uri.getPort() == API_PORT;
    }

    public static File getInstallDir(Context context) {
        return new File(context.getFilesDir(), "inertia");
    }

    private static File getApiBinary(Context context) {
        String libDir = context.getApplicationInfo().nativeLibraryDir;
        return new File(libDir, NATIVE_LIB_NAME);
    }

    /** Installed native lib dir, or a previously staged copy in code_cache. */
    private static File resolveInstalledApiBinary(Context context) {
        File staged = new File(context.getCodeCacheDir(), NATIVE_LIB_NAME);
        if (staged.exists() && staged.canExecute()) {
            return staged;
        }
        File fromLibDir = getApiBinary(context);
        if (fromLibDir.exists() && fromLibDir.canExecute()) {
            return fromLibDir;
        }
        if (staged.exists()) {
            return staged;
        }
        if (fromLibDir.exists()) {
            return fromLibDir;
        }
        return null;
    }

    /** Stage bundled API into code_cache so ProcessBuilder can exec it (works with extractNativeLibs=false). */
    private static File resolveApiBinary(Context context) throws IOException {
        File installed = resolveInstalledApiBinary(context);
        if (installed != null && installed.canExecute()) {
            return installed;
        }
        if (!apkContainsBundledApi(context)) {
            return null;
        }
        File staged = new File(context.getCodeCacheDir(), NATIVE_LIB_NAME);
        copyApiFromApk(context, staged);
        if (!staged.setExecutable(true, false)) {
            throw new IOException("Could not mark bundled API executable");
        }
        return staged;
    }

    private static boolean apkContainsBundledApi(Context context) {
        String abi = Build.SUPPORTED_ABIS.length > 0 ? Build.SUPPORTED_ABIS[0] : "arm64-v8a";
        String entryName = "lib/" + abi + "/" + NATIVE_LIB_NAME;
        try (ZipFile zip = new ZipFile(context.getApplicationInfo().sourceDir)) {
            ZipEntry entry = zip.getEntry(entryName);
            if (entry != null) {
                return true;
            }
            // Fallback when primaryCpuAbi differs from packaged folder name.
            entry = zip.getEntry("lib/arm64-v8a/" + NATIVE_LIB_NAME);
            return entry != null;
        } catch (IOException e) {
            return false;
        }
    }

    private static void copyApiFromApk(Context context, File dest) throws IOException {
        String[] candidates = {
            "lib/" + (Build.SUPPORTED_ABIS.length > 0 ? Build.SUPPORTED_ABIS[0] : "arm64-v8a") + "/" + NATIVE_LIB_NAME,
            "lib/arm64-v8a/" + NATIVE_LIB_NAME,
        };
        try (ZipFile zip = new ZipFile(context.getApplicationInfo().sourceDir)) {
            ZipEntry entry = null;
            for (String candidate : candidates) {
                if (candidate.contains("null")) {
                    continue;
                }
                entry = zip.getEntry(candidate);
                if (entry != null) {
                    break;
                }
            }
            if (entry == null) {
                throw new IOException("Bundled API missing from APK");
            }
            try (InputStream in = zip.getInputStream(entry);
                 OutputStream out = new FileOutputStream(dest)) {
                copyStream(in, out);
            }
        }
    }

    public static synchronized void ensureExtracted(Context context) throws IOException {
        File installDir = getInstallDir(context);
        File versionFile = new File(installDir, ".version");
        File webDir = new File(installDir, "web");
        String expected = getWebBundleVersion(context);
        if (isExtractedWebCurrent(context, versionFile, webDir, expected)) {
            return;
        }
        deleteRecursive(installDir);
        installDir.mkdirs();
        copyAssetTree(context, ASSET_WEB_DIR, webDir);
        writeUtf8(versionFile, expected);
    }

    /** Skip re-extract only when version + packaged web bundle-id match extracted files. */
    private static boolean isExtractedWebCurrent(
        Context context,
        File versionFile,
        File webDir,
        String expectedVersion
    ) {
        if (!versionFile.exists() || !webDir.isDirectory()) {
            return false;
        }
        try {
            if (!expectedVersion.equals(readUtf8(versionFile).trim())) {
                return false;
            }
        } catch (IOException e) {
            return false;
        }
        String assetBundleId = readAssetBundleId(context);
        if (assetBundleId.isEmpty()) {
            return false;
        }
        File extractedBundleId = new File(webDir, EXTRACTED_BUNDLE_ID);
        if (!extractedBundleId.isFile()) {
            return false;
        }
        try {
            return assetBundleId.equals(readUtf8(extractedBundleId).trim());
        } catch (IOException e) {
            return false;
        }
    }

    public static synchronized void start(Context context) throws IOException {
        if (process != null && process.isAlive()) {
            return;
        }
        File binary = resolveApiBinary(context);
        if (binary == null) {
            throw new IOException("Bundled API not packaged in this APK — run npm run android:stage-b and reinstall");
        }
        ensureExtracted(context);
        File installDir = getInstallDir(context);
        File dataDir = new File(installDir, "data");
        if (!dataDir.mkdirs() && !dataDir.isDirectory()) {
            throw new IOException("Could not create data directory");
        }

        ProcessBuilder builder = new ProcessBuilder(binary.getAbsolutePath());
        builder.directory(installDir);
        builder.redirectErrorStream(true);
        Map<String, String> env = builder.environment();
        env.put("INERTIA_DATA_DIR", dataDir.getAbsolutePath());
        env.put("INERTIA_WEB_DIR", new File(installDir, "web").getAbsolutePath());
        env.put("INERTIA_API_ADDR", API_ADDR);
        env.put("RUST_LOG", "info");

        process = builder.start();
        startLogDrain(process);
    }

    public static boolean isHealthy() {
        HttpURLConnection connection = null;
        try {
            connection = (HttpURLConnection) new URL(HEALTH_URL).openConnection();
            connection.setConnectTimeout(750);
            connection.setReadTimeout(750);
            connection.setRequestMethod("GET");
            if (connection.getResponseCode() != 200) {
                return false;
            }
            try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(connection.getInputStream()))) {
                String body = reader.readLine();
                return "ok".equals(body);
            }
        } catch (IOException ignored) {
            return false;
        } finally {
            if (connection != null) {
                connection.disconnect();
            }
        }
    }

    public static void waitForHealthy(long timeoutMs) throws InterruptedException {
        long deadline = System.currentTimeMillis() + timeoutMs;
        while (System.currentTimeMillis() < deadline) {
            if (isHealthy()) {
                return;
            }
            Thread.sleep(200L);
        }
    }

    public static synchronized void stop() {
        if (process != null) {
            process.destroy();
            process = null;
        }
    }

    private static void startLogDrain(Process proc) {
        Thread t = new Thread(() -> {
            try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(proc.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    android.util.Log.i("inertia-api", line);
                }
            } catch (IOException ignored) {
            }
        }, "inertia-api-log");
        t.setDaemon(true);
        t.start();
    }

    private static String getWebBundleVersion(Context context) {
        String app = getAppVersion(context);
        String bundleId = readAssetBundleId(context);
        if (!bundleId.isEmpty()) {
            return app + "+" + bundleId;
        }
        return app;
    }

    private static String readAssetBundleId(Context context) {
        try {
            return readAssetUtf8(context, ASSET_BUNDLE_ID).trim();
        } catch (IOException ignored) {
            return "";
        }
    }

    private static String readAssetUtf8(Context context, String assetPath) throws IOException {
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(
            context.getAssets().open(assetPath)))) {
            String line = reader.readLine();
            return line != null ? line : "";
        }
    }

    private static String getAppVersion(Context context) {
        try {
            PackageInfo info = context.getPackageManager()
                .getPackageInfo(context.getPackageName(), 0);
            long code = Build.VERSION.SDK_INT >= Build.VERSION_CODES.P
                ? info.getLongVersionCode()
                : info.versionCode;
            return info.versionName + "+" + code;
        } catch (PackageManager.NameNotFoundException e) {
            return "unknown";
        }
    }

    private static void copyAssetFile(Context context, String assetPath, File dest) throws IOException {
        try (InputStream in = context.getAssets().open(assetPath);
             OutputStream out = new FileOutputStream(dest)) {
            copyStream(in, out);
        }
    }

    private static void copyAssetTree(Context context, String assetDir, File destDir) throws IOException {
        String[] children = context.getAssets().list(assetDir);
        if (children == null || children.length == 0) {
            destDir.getParentFile().mkdirs();
            copyAssetFile(context, assetDir, destDir);
            return;
        }
        destDir.mkdirs();
        for (String child : children) {
            String childAsset = assetDir + "/" + child;
            String[] nested = context.getAssets().list(childAsset);
            File childDest = new File(destDir, child);
            if (nested != null && nested.length > 0) {
                copyAssetTree(context, childAsset, childDest);
            } else {
                copyAssetFile(context, childAsset, childDest);
            }
        }
    }

    private static void copyStream(InputStream in, OutputStream out) throws IOException {
        byte[] buffer = new byte[8192];
        int read;
        while ((read = in.read(buffer)) != -1) {
            out.write(buffer, 0, read);
        }
    }

    private static String readUtf8(File file) throws IOException {
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(
            new java.io.FileInputStream(file)))) {
            return reader.readLine();
        }
    }

    private static void writeUtf8(File file, String text) throws IOException {
        try (FileOutputStream out = new FileOutputStream(file)) {
            out.write(text.getBytes(java.nio.charset.StandardCharsets.UTF_8));
        }
    }

    /** Map inertia://, on-device http, or dev localhost invite links to the Stage B WebView URL. */
    public static String inviteIntentToUiUrl(Intent intent) {
        if (intent == null) {
            return null;
        }
        return anyInviteToUiUrl(intent.getData());
    }

    public static String anyInviteToUiUrl(Uri uri) {
        if (uri == null) {
            return null;
        }
        if ("inertia".equals(uri.getScheme()) && "invite".equals(uri.getHost())) {
            String payload = uri.getPath();
            if (payload != null && payload.startsWith("/")) {
                payload = payload.substring(1);
            }
            if (payload == null || payload.isEmpty()) {
                return null;
            }
            return UI_URL + "invite#" + payload;
        }
        if ("http".equals(uri.getScheme())
            && API_HOST.equals(uri.getHost())
            && uri.getPort() == API_PORT) {
            String path = uri.getPath();
            if (path == null || !path.startsWith("/invite")) {
                return null;
            }
            String fragment = uri.getFragment();
            if (fragment == null || fragment.isEmpty()) {
                return null;
            }
            return UI_URL + "invite#" + fragment;
        }
        return extractInviteFragment(uri);
    }

    /** localhost / LAN dev links — keep invite handling inside the app WebView. */
    private static String extractInviteFragment(Uri uri) {
        if (uri == null) {
            return null;
        }
        String path = uri.getPath();
        if (path == null || !path.startsWith("/invite")) {
            return null;
        }
        String fragment = uri.getFragment();
        if (fragment == null || fragment.isEmpty()) {
            return null;
        }
        return UI_URL + "invite#" + fragment;
    }

    private static void deleteRecursive(File file) {
        if (file.isDirectory()) {
            File[] children = file.listFiles();
            if (children != null) {
                for (File child : children) {
                    deleteRecursive(child);
                }
            }
        }
        file.delete();
    }
}
