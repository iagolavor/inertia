package social.inertia.app;

import android.content.ContentResolver;
import android.content.ContentValues;
import android.content.Context;
import android.media.MediaScannerConnection;
import android.net.Uri;
import android.os.Build;
import android.os.Environment;
import android.provider.MediaStore;
import android.util.Log;
import android.webkit.URLUtil;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.function.Consumer;

/** Fetch from the on-device inertia-api and write into public Downloads. */
public final class LocalDownloadSaver {
    private static final String TAG = "Inertia";
    private static final ExecutorService EXECUTOR = Executors.newSingleThreadExecutor();

    private final Context context;

    public LocalDownloadSaver(Context context) {
        this.context = context.getApplicationContext();
    }

    public void saveFromUrl(String urlString, String suggestedFileName, Consumer<Boolean> done) {
        EXECUTOR.execute(() -> {
            boolean ok = false;
            try {
                if (urlString != null && urlString.startsWith("blob:")) {
                    throw new IOException("blob URLs are not supported");
                }
                ok = saveFromUrlBlocking(urlString, suggestedFileName);
            } catch (Exception e) {
                Log.e(TAG, "local download failed", e);
            }
            boolean result = ok;
            if (done != null) {
                done.accept(result);
            }
        });
    }

    public void saveBase64(String suggestedFileName, String mimeType, String dataBase64, Consumer<Boolean> done) {
        EXECUTOR.execute(() -> {
            boolean ok = false;
            try {
                byte[] data = android.util.Base64.decode(dataBase64, android.util.Base64.DEFAULT);
                String name = sanitizeFileName(suggestedFileName);
                if (name.isEmpty()) {
                    name = "inertia-download";
                }
                String mime = mimeType == null || mimeType.isEmpty()
                    ? guessMime(name)
                    : mimeType;
                writeToDownloads(name, mime, data);
                Log.i(TAG, "saved base64 download " + name + " (" + data.length + " bytes)");
                ok = true;
            } catch (Exception e) {
                Log.e(TAG, "base64 download failed", e);
            }
            if (done != null) {
                done.accept(ok);
            }
        });
    }

    private boolean saveFromUrlBlocking(String urlString, String suggestedFileName) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) new URL(urlString).openConnection();
        connection.setConnectTimeout(15_000);
        connection.setReadTimeout(120_000);
        connection.setRequestMethod("GET");
        connection.setInstanceFollowRedirects(true);

        try {
            int code = connection.getResponseCode();
            if (code != HttpURLConnection.HTTP_OK) {
                throw new IOException("HTTP " + code);
            }

            String mime = connection.getContentType();
            if (mime != null) {
                int semi = mime.indexOf(';');
                if (semi >= 0) {
                    mime = mime.substring(0, semi).trim();
                }
            }
            if (mime == null || mime.isEmpty()) {
                mime = guessMime(suggestedFileName);
            }

            String name = sanitizeFileName(suggestedFileName);
            if (name.isEmpty()) {
                name = URLUtil.guessFileName(
                    urlString,
                    connection.getHeaderField("Content-Disposition"),
                    mime
                );
            }
            name = sanitizeFileName(name);
            if (name.isEmpty()) {
                name = "inertia-download";
            }

            byte[] data = readAll(connection.getInputStream());
            writeToDownloads(name, mime, data);
            Log.i(TAG, "saved download " + name + " (" + data.length + " bytes)");
            return true;
        } finally {
            connection.disconnect();
        }
    }

    private void writeToDownloads(String name, String mime, byte[] data) throws IOException {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ContentResolver resolver = context.getContentResolver();
            ContentValues pending = new ContentValues();
            pending.put(MediaStore.MediaColumns.DISPLAY_NAME, name);
            pending.put(MediaStore.MediaColumns.MIME_TYPE, mime);
            pending.put(MediaStore.MediaColumns.RELATIVE_PATH, Environment.DIRECTORY_DOWNLOADS);
            pending.put(MediaStore.MediaColumns.IS_PENDING, 1);

            Uri uri = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, pending);
            if (uri == null) {
                throw new IOException("MediaStore insert failed");
            }

            try (OutputStream out = resolver.openOutputStream(uri)) {
                if (out == null) {
                    throw new IOException("MediaStore openOutputStream failed");
                }
                out.write(data);
            }

            ContentValues complete = new ContentValues();
            complete.put(MediaStore.MediaColumns.IS_PENDING, 0);
            resolver.update(uri, complete, null, null);
            return;
        }

        File dir = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS);
        if (!dir.exists() && !dir.mkdirs()) {
            throw new IOException("Downloads folder unavailable");
        }
        File file = new File(dir, name);
        try (FileOutputStream out = new FileOutputStream(file)) {
            out.write(data);
        }
        MediaScannerConnection.scanFile(
            context,
            new String[] { file.getAbsolutePath() },
            new String[] { mime },
            null
        );
    }

    private static byte[] readAll(InputStream in) throws IOException {
        ByteArrayOutputStream buffer = new ByteArrayOutputStream();
        byte[] chunk = new byte[8192];
        int read;
        while ((read = in.read(chunk)) != -1) {
            buffer.write(chunk, 0, read);
        }
        return buffer.toByteArray();
    }

    private static String sanitizeFileName(String name) {
        if (name == null) {
            return "";
        }
        String trimmed = name.trim();
        if (trimmed.isEmpty()) {
            return "";
        }
        StringBuilder out = new StringBuilder(trimmed.length());
        for (int i = 0; i < trimmed.length(); i++) {
            char c = trimmed.charAt(i);
            if (c == '/' || c == '\\' || c == '\r' || c == '\n') {
                out.append('_');
            } else {
                out.append(c);
            }
        }
        return out.toString();
    }

    private static String guessMime(String name) {
        String lower = name == null ? "" : name.toLowerCase();
        if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) {
            return "image/jpeg";
        }
        if (lower.endsWith(".png")) {
            return "image/png";
        }
        if (lower.endsWith(".gif")) {
            return "image/gif";
        }
        if (lower.endsWith(".webp")) {
            return "image/webp";
        }
        if (lower.endsWith(".zip")) {
            return "application/zip";
        }
        return "application/octet-stream";
    }
}
