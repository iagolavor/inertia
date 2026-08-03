package social.inertia.app;

import android.app.Activity;
import android.content.Intent;
import android.os.Build;
import android.os.Bundle;

import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;

/** Boots bundled on-device API, then opens Tauri MainActivity when healthy. */
public class SplashActivity extends Activity {
    private static final long HEALTH_TIMEOUT_MS = 45_000L;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        String pendingInviteUrl = InertiaRuntime.inviteIntentToUiUrl(getIntent());
        if (pendingInviteUrl != null) {
            writePendingInvite(pendingInviteUrl);
        }
        setContentView(R.layout.activity_splash);
        new Thread(this::bootAndOpenMain).start();
    }

    private void writePendingInvite(String url) {
        try {
            File file = new File(getFilesDir(), "pending-invite-url");
            try (FileOutputStream out = new FileOutputStream(file)) {
                out.write(url.getBytes(StandardCharsets.UTF_8));
            }
        } catch (Exception e) {
            android.util.Log.w("Inertia", "failed to write pending invite", e);
        }
    }

    private void bootAndOpenMain() {
        boolean bundledApi = InertiaRuntime.hasBundledApi(this);
        if (bundledApi) {
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
            main.putExtra(MainActivity.EXTRA_BUNDLED_API, bundledApi);
            startActivity(main);
            finish();
        });
    }
}
