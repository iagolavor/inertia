package social.inertia.app;

import android.net.Uri;
import android.webkit.WebResourceRequest;
import android.webkit.WebView;
import android.webkit.WebViewClient;

/**
 * Keep invite links and the on-device API origin inside the app WebView.
 * External http(s) still follows the default WebView / system handler.
 */
public final class InertiaWebViewClient extends WebViewClient {
    private final WebViewClient upstream;
    private final Runnable onPageReady;

    public InertiaWebViewClient(WebViewClient upstream, Runnable onPageReady) {
        this.upstream = upstream;
        this.onPageReady = onPageReady;
    }

    @Override
    public void onPageFinished(WebView view, String url) {
        if (upstream != null) {
            upstream.onPageFinished(view, url);
        } else {
            super.onPageFinished(view, url);
        }
        if (onPageReady != null) {
            onPageReady.run();
        }
    }

    @Override
    public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
        Uri url = request.getUrl();
        String inviteUi = InertiaRuntime.anyInviteToUiUrl(url);
        if (inviteUi != null) {
            view.loadUrl(inviteUi);
            return true;
        }
        if (InertiaRuntime.isInertiaApiUrl(url)) {
            return false;
        }
        if (upstream != null) {
            return upstream.shouldOverrideUrlLoading(view, request);
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
        if (upstream != null) {
            return upstream.shouldOverrideUrlLoading(view, url);
        }
        return super.shouldOverrideUrlLoading(view, url);
    }
}
