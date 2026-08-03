package social.inertia.app

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.webkit.WebSettings
import android.webkit.WebView
import android.widget.Toast
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import java.util.Locale

class MainActivity : TauriActivity() {
  companion object {
    const val EXTRA_BUNDLED_API = "bundled_api"
  }

  private var bundledApi = false
  private var downloadSaver: LocalDownloadSaver? = null
  private var safeInsetLeft = 0
  private var safeInsetTop = 0
  private var safeInsetRight = 0
  private var safeInsetBottom = 0

  override fun onCreate(savedInstanceState: Bundle?) {
    bundledApi = resolveBundledApi(intent)
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    downloadSaver = LocalDownloadSaver(this)

    val window = window
    WindowCompat.setDecorFitsSystemWindows(window, false)
    window.statusBarColor = Color.TRANSPARENT
    window.navigationBarColor = Color.TRANSPARENT

    val midnight = Color.parseColor("#08090c")
    window.decorView.setBackgroundColor(midnight)

    if (bundledApi) {
      ensureApiServiceRunning()
    }

    // Tauri may create the WebView after the first layout pass; retry briefly.
    scheduleWebViewSetup(midnight, 0)
  }

  private fun scheduleWebViewSetup(midnight: Int, attempt: Int) {
    window.decorView.post {
      val webView = findWebView(window.decorView)
      if (webView == null) {
        if (attempt < 50) {
          window.decorView.postDelayed({ scheduleWebViewSetup(midnight, attempt + 1) }, 100L)
        } else {
          android.util.Log.e("Inertia", "WebView not found after retries")
        }
        return@post
      }
      webView.setBackgroundColor(midnight)
      // Keep invite + on-device API navigations inside the WebView (Cap parity).
      webView.settings.mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
      val upstream = webView.webViewClient
      webView.webViewClient = InertiaWebViewClient(upstream) {
        injectSafeAreaCss(webView)
      }
      installSafeAreaInsets(webView)
      webView.addJavascriptInterface(InertiaDownloadBridge(this), "InertiaDownload")
      webView.setDownloadListener { url, _, contentDisposition, _, _ ->
        if (url != null && url.startsWith("blob:")) {
          android.util.Log.w("Inertia", "ignoring blob download URL")
          return@setDownloadListener
        }
        startLocalDownload(url, fileNameFromDisposition(contentDisposition))
      }
      loadUiIntoWebView(webView)
    }
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    setIntent(intent)
    bundledApi = resolveBundledApi(intent)
    if (bundledApi) {
      ensureApiServiceRunning()
    }
    val invite = InertiaRuntime.inviteIntentToUiUrl(intent)
    if (invite != null) {
      try {
        val file = java.io.File(filesDir, "pending-invite-url")
        file.writeText(invite)
      } catch (e: Exception) {
        android.util.Log.w("Inertia", "failed to write pending invite", e)
      }
      findWebView(window.decorView)?.loadUrl(invite)
    }
  }

  fun enqueueDownloadFromJs(url: String?, suggestedFileName: String?) {
    startLocalDownload(url, suggestedFileName)
  }

  fun saveBase64FromJs(suggestedFileName: String?, mimeType: String?, dataBase64: String?) {
    Toast.makeText(this, "Saving to Downloads…", Toast.LENGTH_SHORT).show()
    downloadSaver?.saveBase64(suggestedFileName, mimeType, dataBase64) { ok ->
      runOnUiThread {
        if (ok) {
          Toast.makeText(this, "Saved to Downloads", Toast.LENGTH_LONG).show()
        } else {
          Toast.makeText(this, "Download failed", Toast.LENGTH_SHORT).show()
        }
      }
    }
  }

  private fun startLocalDownload(url: String?, suggestedFileName: String?) {
    if (url.isNullOrEmpty()) {
      Toast.makeText(this, "Download failed", Toast.LENGTH_SHORT).show()
      return
    }
    Toast.makeText(this, "Saving to Downloads…", Toast.LENGTH_SHORT).show()
    downloadSaver?.saveFromUrl(url, suggestedFileName) { ok ->
      runOnUiThread {
        if (ok) {
          Toast.makeText(this, "Saved to Downloads", Toast.LENGTH_LONG).show()
        } else {
          Toast.makeText(this, "Download failed", Toast.LENGTH_SHORT).show()
        }
      }
    }
  }

  private fun installSafeAreaInsets(webView: WebView) {
    ViewCompat.setOnApplyWindowInsetsListener(webView) { v, windowInsets ->
      var source = ViewCompat.getRootWindowInsets(v)
      if (source == null) {
        source = windowInsets
      }
      val bars = source.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      )
      val ime = source.getInsets(WindowInsetsCompat.Type.ime())
      safeInsetLeft = bars.left
      safeInsetTop = bars.top
      safeInsetRight = bars.right
      safeInsetBottom = maxOf(bars.bottom, ime.bottom)
      injectSafeAreaCss(webView)
      windowInsets
    }
    ViewCompat.requestApplyInsets(webView)
  }

  private fun injectSafeAreaCss(webView: WebView) {
    var density = webView.resources.displayMetrics.density
    if (density <= 0f) {
      density = 1f
    }
    val js = String.format(
      Locale.US,
      "(function(){var r=document.documentElement;" +
        "r.style.setProperty('--safe-top','%.2fpx');" +
        "r.style.setProperty('--safe-right','%.2fpx');" +
        "r.style.setProperty('--safe-bottom','%.2fpx');" +
        "r.style.setProperty('--safe-left','%.2fpx');" +
        "})();",
      safeInsetTop / density,
      safeInsetRight / density,
      safeInsetBottom / density,
      safeInsetLeft / density
    )
    webView.evaluateJavascript(js, null)
  }

  private fun loadUiIntoWebView(webView: WebView) {
    val url = resolveStartUrl()
    android.util.Log.i("Inertia", "loading UI $url")
    webView.loadUrl(url)
  }

  private fun resolveStartUrl(): String {
    val fromIntent = InertiaRuntime.inviteIntentToUiUrl(intent)
    if (fromIntent != null) {
      return fromIntent
    }
    val pending = java.io.File(filesDir, "pending-invite-url")
    if (pending.isFile) {
      try {
        val text = pending.readText().trim()
        pending.delete()
        if (text.isNotEmpty()) {
          return text
        }
      } catch (e: Exception) {
        android.util.Log.w("Inertia", "failed to read pending invite", e)
      }
    }
    return InertiaRuntime.getUiUrl()
  }

  private fun resolveBundledApi(intent: Intent?): Boolean {
    if (intent != null && intent.hasExtra(EXTRA_BUNDLED_API)) {
      return intent.getBooleanExtra(EXTRA_BUNDLED_API, false)
    }
    return InertiaRuntime.hasBundledApi(this)
  }

  private fun ensureApiServiceRunning() {
    val service = Intent(this, InertiaApiService::class.java)
    if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
      startForegroundService(service)
    } else {
      startService(service)
    }
  }

  private fun findWebView(root: android.view.View?): WebView? {
    if (root == null) return null
    if (root is WebView) return root
    if (root is android.view.ViewGroup) {
      for (i in 0 until root.childCount) {
        val found = findWebView(root.getChildAt(i))
        if (found != null) return found
      }
    }
    return null
  }

  private fun fileNameFromDisposition(contentDisposition: String?): String {
    if (contentDisposition == null) return ""
    val marker = "filename="
    val idx = contentDisposition.indexOf(marker)
    if (idx < 0) return ""
    var raw = contentDisposition.substring(idx + marker.length).trim()
    if (raw.startsWith("\"") && raw.endsWith("\"") && raw.length >= 2) {
      raw = raw.substring(1, raw.length - 1)
    }
    return raw
  }
}
