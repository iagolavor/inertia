package social.inertia.app;

import android.os.Bundle;
import android.graphics.Color;
import android.content.Intent;
import android.view.Window;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.widget.Toast;

import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsControllerCompat;

import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    public static final String EXTRA_BUNDLED_API = "bundled_api";
    public static final String EXTRA_INVITE_URL = "invite_url";

    private boolean bundledApi;
    private String inviteLoadUrl;
    private LocalDownloadSaver downloadSaver;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        bundledApi = resolveBundledApi(getIntent());
        captureInviteUrl(getIntent());
        super.onCreate(savedInstanceState);

        downloadSaver = new LocalDownloadSaver(this);

        Window window = getWindow();
        WindowCompat.setDecorFitsSystemWindows(window, false);
        window.setStatusBarColor(Color.TRANSPARENT);
        window.setNavigationBarColor(Color.TRANSPARENT);

        WindowInsetsControllerCompat insetsController =
            WindowCompat.getInsetsController(window, window.getDecorView());
        insetsController.setAppearanceLightStatusBars(false);
        insetsController.setAppearanceLightNavigationBars(false);

        WebSettings settings = getBridge().getWebView().getSettings();
        settings.setMixedContentMode(WebSettings.MIXED_CONTENT_ALWAYS_ALLOW);
        WebView webView = getBridge().getWebView();
        // Midnight surface: avoids white flash behind ActionMode / selection UI.
        int midnight = Color.parseColor("#08090c");
        webView.setBackgroundColor(midnight);
        webView.setBackground(new android.graphics.drawable.ColorDrawable(midnight));
        getWindow().getDecorView().setBackgroundColor(midnight);
        webView.addJavascriptInterface(new InertiaDownloadBridge(this), "InertiaDownload");
        webView.setDownloadListener(
            (url, userAgent, contentDisposition, mimeType, contentLength) -> {
                if (url != null && url.startsWith("blob:")) {
                    android.util.Log.w("Inertia", "ignoring blob download URL");
                    return;
                }
                startLocalDownload(url, fileNameFromDisposition(contentDisposition));
            }
        );
        getBridge().setWebViewClient(new InertiaWebViewClient(getBridge()));

        if (bundledApi) {
            ensureApiServiceRunning();
            ensureBundledWebUrl();
        }
    }

    /** Called from InertiaDownloadBridge (WebView ignores anchor download clicks). */
    void enqueueDownloadFromJs(String url, String suggestedFileName) {
        startLocalDownload(url, suggestedFileName);
    }

    void saveBase64FromJs(String suggestedFileName, String mimeType, String dataBase64) {
        Toast.makeText(this, "Saving to Downloads…", Toast.LENGTH_SHORT).show();
        downloadSaver.saveBase64(suggestedFileName, mimeType, dataBase64, ok -> runOnUiThread(() -> {
            if (ok) {
                Toast.makeText(this, "Saved to Downloads", Toast.LENGTH_LONG).show();
            } else {
                Toast.makeText(this, "Download failed", Toast.LENGTH_SHORT).show();
            }
        }));
    }

    private void startLocalDownload(String url, String suggestedFileName) {
        if (url == null || url.isEmpty()) {
            Toast.makeText(this, "Download failed", Toast.LENGTH_SHORT).show();
            return;
        }
        Toast.makeText(this, "Saving to Downloads…", Toast.LENGTH_SHORT).show();
        downloadSaver.saveFromUrl(url, suggestedFileName, ok -> runOnUiThread(() -> {
            if (ok) {
                Toast.makeText(
                    this,
                    "Saved to Downloads",
                    Toast.LENGTH_LONG
                ).show();
            } else {
                Toast.makeText(this, "Download failed", Toast.LENGTH_SHORT).show();
            }
        }));
    }

    private static String fileNameFromDisposition(String contentDisposition) {
        if (contentDisposition == null) {
            return "";
        }
        String marker = "filename=";
        int idx = contentDisposition.indexOf(marker);
        if (idx < 0) {
            return "";
        }
        String raw = contentDisposition.substring(idx + marker.length()).trim();
        if (raw.startsWith("\"") && raw.endsWith("\"") && raw.length() >= 2) {
            raw = raw.substring(1, raw.length() - 1);
        }
        return raw;
    }

    private void ensureBundledWebUrl() {
        WebView webView = getBridge().getWebView();
        String target = inviteLoadUrl != null ? inviteLoadUrl : InertiaRuntime.getUiUrl();
        inviteLoadUrl = null;
        String current = webView.getUrl();

        if (shouldLoadBundledWebUrl(current, target)) {
            webView.loadUrl(target);
        }
    }

    /** Only replace Capacitor localhost shell or first load — do not clobber in-app navigation. */
    private boolean shouldLoadBundledWebUrl(String current, String target) {
        if (current == null || current.isEmpty() || "about:blank".equals(current)) {
            return true;
        }
        if (current.contains("localhost") && !current.contains("127.0.0.1:4783")) {
            return true;
        }
        if (!current.startsWith("http://127.0.0.1:4783")) {
            return true;
        }
        if (target.contains("#") && !current.equals(target)) {
            return true;
        }
        return false;
    }

    private void captureInviteUrl(Intent intent) {
        if (intent == null) {
            return;
        }
        if (intent.hasExtra(EXTRA_INVITE_URL)) {
            inviteLoadUrl = intent.getStringExtra(EXTRA_INVITE_URL);
            return;
        }
        inviteLoadUrl = InertiaRuntime.inviteIntentToUiUrl(intent);
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        bundledApi = resolveBundledApi(intent);
        captureInviteUrl(intent);
        if (bundledApi) {
            ensureApiServiceRunning();
            ensureBundledWebUrl();
        }
    }

    private boolean resolveBundledApi(Intent intent) {
        if (intent != null && intent.hasExtra(EXTRA_BUNDLED_API)) {
            return intent.getBooleanExtra(EXTRA_BUNDLED_API, false);
        }
        return InertiaRuntime.hasBundledApi(this);
    }

    private void ensureApiServiceRunning() {
        Intent service = new Intent(this, InertiaApiService.class);
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
            startForegroundService(service);
        } else {
            startService(service);
        }
    }
}
