package social.inertia.app;

import android.webkit.JavascriptInterface;

/** JS bridge: WebView ignores programmatic anchor download clicks for same-origin URLs. */
public final class InertiaDownloadBridge {
    private final MainActivity host;

    public InertiaDownloadBridge(MainActivity host) {
        this.host = host;
    }

    @JavascriptInterface
    public void enqueue(String url, String suggestedFileName) {
        host.runOnUiThread(() -> host.enqueueDownloadFromJs(url, suggestedFileName));
    }

    @JavascriptInterface
    public void saveBase64(String suggestedFileName, String mimeType, String dataBase64) {
        host.runOnUiThread(() -> host.saveBase64FromJs(suggestedFileName, mimeType, dataBase64));
    }
}
