package social.inertia.app;

import android.content.Intent;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.view.Window;
import android.webkit.WebSettings;
import android.webkit.WebView;

import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsControllerCompat;

import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    public static final String EXTRA_STAGE_B = "stage_b";
    public static final String EXTRA_INVITE_URL = "invite_url";

    private boolean stageB;
    private String inviteLoadUrl;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        stageB = resolveStageB(getIntent());
        captureInviteUrl(getIntent());
        super.onCreate(savedInstanceState);

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
        getBridge().getWebView().setBackgroundColor(Color.parseColor("#08090c"));
        getBridge().setWebViewClient(new InertiaWebViewClient(getBridge()));

        if (stageB) {
            ensureStageBApi();
            ensureStageBWebUrl();
        }
    }

    private void ensureStageBWebUrl() {
        WebView webView = getBridge().getWebView();
        String target = inviteLoadUrl != null ? inviteLoadUrl : InertiaRuntime.getUiUrl();
        inviteLoadUrl = null;
        String current = webView.getUrl();

        if (shouldLoadStageBUrl(current, target)) {
            webView.loadUrl(target);
        }
    }

    /** Only replace Capacitor localhost shell or first load — do not clobber in-app navigation. */
    private boolean shouldLoadStageBUrl(String current, String target) {
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
        stageB = resolveStageB(intent);
        captureInviteUrl(intent);
        if (stageB) {
            ensureStageBApi();
            ensureStageBWebUrl();
        }
    }

    private boolean resolveStageB(Intent intent) {
        if (intent != null && intent.hasExtra(EXTRA_STAGE_B)) {
            return intent.getBooleanExtra(EXTRA_STAGE_B, false);
        }
        return InertiaRuntime.hasBundledApi(this);
    }

    private void ensureStageBApi() {
        Intent service = new Intent(this, InertiaApiService.class);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(service);
        } else {
            startService(service);
        }
    }
}
