package social.inertia.app;

import android.net.Uri;
import android.webkit.WebResourceRequest;
import android.webkit.WebView;

import com.getcapacitor.Bridge;
import com.getcapacitor.BridgeWebViewClient;

/**
 * Keep invite links and the Stage B API origin inside the app — Capacitor opens other origins in Chrome.
 */
public final class InertiaWebViewClient extends BridgeWebViewClient {

    public InertiaWebViewClient(Bridge bridge) {
        super(bridge);
    }

    @Override
    public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
        Uri url = request.getUrl();
        String inviteUi = InertiaRuntime.anyInviteToUiUrl(url);
        if (inviteUi != null) {
            view.loadUrl(inviteUi);
            return true;
        }
        // WebView loads the device API in-place; do not call super (opens Chrome).
        if (InertiaRuntime.isInertiaApiUrl(url)) {
            return false;
        }
        return super.shouldOverrideUrlLoading(view, request);
    }

    @Deprecated
    @Override
    public boolean shouldOverrideUrlLoading(WebView view, String url) {
        Uri uri = Uri.parse(url);
        String inviteUi = InertiaRuntime.anyInviteToUiUrl(uri);
        if (inviteUi != null) {
            view.loadUrl(inviteUi);
            return true;
        }
        if (InertiaRuntime.isInertiaApiUrl(uri)) {
            return false;
        }
        return super.shouldOverrideUrlLoading(view, url);
    }
}
