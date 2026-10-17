package io.github.serhanensar.ec_android

import android.accessibilityservice.AccessibilityService
import android.content.ComponentName
import android.content.Context
import android.graphics.Bitmap
import android.os.Build
import android.provider.Settings
import android.util.Log
import android.view.Display
import android.view.accessibility.AccessibilityEvent
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Optional accessibility service behind "screen access". It only remembers which app
 * is in front; it reads the browser's address bar or takes a screenshot only when the
 * user holds the fling handle. Nothing is sent anywhere except by an explicit fling.
 */
class ScreenAccessService : AccessibilityService() {
    @Volatile private var foreground: String? = null

    override fun onServiceConnected() {
        instance = this
    }

    override fun onUnbind(intent: android.content.Intent?): Boolean {
        instance = null
        return super.onUnbind(intent)
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent) {
        if (event.eventType != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return
        val pkg = event.packageName?.toString() ?: return
        // Our own overlay and the system UI are not "what is on screen".
        if (pkg != packageName && pkg != "com.android.systemui") foreground = pkg
    }

    override fun onInterrupt() {}

    /** The web page open in the browser in front, as (url, title), if any. */
    fun currentPage(): Pair<String, String>? {
        Log.d(TAG, "in front: $foreground")
        val ids = BROWSER_URL_BARS[foreground ?: return null] ?: return null
        val root = rootInActiveWindow ?: return null
        for (id in ids) {
            val text = root.findAccessibilityNodeInfosByViewId(id).firstOrNull()?.text?.toString()
            val url = text?.let(::normalizeUrl) ?: continue
            return url to ""
        }
        return null
    }

    /** Takes a screenshot and saves it as a JPEG in the cache; null on failure. */
    fun capture(done: (String?) -> Unit) {
        if (Build.VERSION.SDK_INT < 30) return done(null)
        takeScreenshot(Display.DEFAULT_DISPLAY, mainExecutor, object : TakeScreenshotCallback {
            override fun onSuccess(result: ScreenshotResult) {
                val path = runCatching {
                    val hardware = Bitmap.wrapHardwareBuffer(result.hardwareBuffer, result.colorSpace)
                    val bitmap = hardware?.copy(Bitmap.Config.ARGB_8888, false)
                    result.hardwareBuffer.close()
                    val dir = File(cacheDir, "fling").apply { mkdirs() }
                    val stamp = SimpleDateFormat("yyyy-MM-dd HH.mm.ss", Locale.ROOT).format(Date())
                    val file = File(dir, "Screenshot $stamp.jpg")
                    file.outputStream().use { bitmap?.compress(Bitmap.CompressFormat.JPEG, 90, it) }
                    file.absolutePath
                }.onFailure { Log.w(TAG, "saving the screenshot failed", it) }.getOrNull()
                done(path)
            }

            override fun onFailure(errorCode: Int) {
                Log.w(TAG, "screenshot failed with code $errorCode")
                done(null)
            }
        })
    }

    companion object {
        private const val TAG = "EcScreenAccess"

        @Volatile var instance: ScreenAccessService? = null
            private set

        /** Address bar view ids of common browsers. */
        private val BROWSER_URL_BARS = mapOf(
            "com.android.chrome" to listOf("com.android.chrome:id/url_bar"),
            "com.brave.browser" to listOf("com.brave.browser:id/url_bar"),
            "com.microsoft.emmx" to listOf("com.microsoft.emmx:id/url_bar"),
            "com.sec.android.app.sbrowser" to listOf(
                "com.sec.android.app.sbrowser:id/location_bar_edit_text",
                "com.sec.android.app.sbrowser:id/sbrowser_url_bar",
            ),
            "org.mozilla.firefox" to listOf("org.mozilla.firefox:id/mozac_browser_toolbar_url_view"),
            "com.opera.browser" to listOf("com.opera.browser:id/url_field"),
        )

        /** Address bars often hide the scheme; search text is not a URL. */
        fun normalizeUrl(text: String): String? {
            val t = text.trim()
            if (t.isEmpty() || t.any { it.isWhitespace() }) return null
            if (t.startsWith("http://") || t.startsWith("https://")) return t
            if (!t.contains('.')) return null
            return "https://$t"
        }

        /** Whether the user turned the service on in the system settings. */
        fun isEnabled(context: Context): Boolean {
            val enabled = Settings.Secure.getString(
                context.contentResolver, Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES,
            ) ?: return false
            val me = ComponentName(context, ScreenAccessService::class.java).flattenToString()
            return enabled.split(':').any { it.equals(me, ignoreCase = true) }
        }
    }
}
