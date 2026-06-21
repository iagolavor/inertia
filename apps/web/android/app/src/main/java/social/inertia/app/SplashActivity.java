package social.inertia.app;

import android.app.Activity;
import android.content.Intent;
import android.os.Build;
import android.os.Bundle;

/** Boots bundled API (Stage B) or passes through to MainActivity (Stage A dev). */
public class SplashActivity extends Activity {
    private static final long HEALTH_TIMEOUT_MS = 45_000L;
    private String pendingInviteUrl;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        pendingInviteUrl = InertiaRuntime.inviteIntentToUiUrl(getIntent());
        setContentView(R.layout.activity_splash);

        new Thread(this::bootAndOpenMain).start();
    }

    private void bootAndOpenMain() {
        boolean stageB = InertiaRuntime.hasBundledApi(this);
        if (stageB) {
            runOnUiThread(() -> {
                Intent service = new Intent(this, InertiaApiService.class);
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    startForegroundService(service);
                } else {
                    startService(service);
                }
            });
            try {
                InertiaRuntime.waitForHealthy(HEALTH_TIMEOUT_MS);
            } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
            }
        }

        runOnUiThread(() -> {
            Intent main = new Intent(this, MainActivity.class);
            main.putExtra(MainActivity.EXTRA_STAGE_B, stageB);
            if (pendingInviteUrl != null) {
                main.putExtra(MainActivity.EXTRA_INVITE_URL, pendingInviteUrl);
            }
            startActivity(main);
            finish();
        });
    }
}
