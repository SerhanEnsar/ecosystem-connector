package io.github.serhanensar.ec_android

import android.accessibilityservice.AccessibilityService
import android.content.ComponentName
import android.content.Context
import android.content.res.Resources
import android.graphics.Bitmap
import android.graphics.Rect
import android.os.Build
import android.provider.Settings
import android.util.Log
import android.view.Display
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Optional accessibility service behind "screen access". It only remembers which app
 * is in front; it copies the selected text, reads the browser's address bar or takes a
 * screenshot only when the user swipes in from the fling handle. Nothing is sent
 * anywhere except by an explicit fling.
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

    /**
     * The window of the app in front. Asked at fling time rather than remembered from
     * events: keyboards, the S Pen menu and edge panels also report window changes, and
     * would otherwise pass for the app in front.
     */
    private fun appRoot(): AccessibilityNodeInfo? {
        val apps = windows.filter {
            it.type == AccessibilityWindowInfo.TYPE_APPLICATION &&
                it.root?.packageName?.toString().let { pkg -> pkg != null && pkg != packageName }
        }
        val front = apps.firstOrNull { it.isActive || it.isFocused } ?: apps.firstOrNull()
        val pkg = front?.root?.packageName
        // The app's own window, not a menu or popup it shows on top (those are windows
        // too, and the address bar is not in them).
        val bounds = Rect()
        val window = apps.filter { it.root?.packageName == pkg }.maxByOrNull {
            it.getBoundsInScreen(bounds)
            bounds.width().toLong() * bounds.height()
        }
        return window?.root ?: rootInActiveWindow?.takeIf { it.packageName != packageName }
    }

    /** The web page open in the browser in front, as (url, title), if any. */
    fun currentPage(): Pair<String, String>? {
        val root = appRoot() ?: return null
        val pkg = root.packageName?.toString() ?: foreground
        Log.d(TAG, "in front: $pkg")
        val ids = BROWSER_URL_BARS[pkg ?: return null] ?: return null
        for (id in ids) {
            val text = root.findAccessibilityNodeInfosByViewId(id).firstOrNull()?.text?.toString()
            val url = text?.let(::normalizeUrl) ?: continue
            return url to ""
        }
        return null
    }

    /**
     * The link whose menu is open in the browser (after a long press on a link), as
     * (url, title), closing the menu. While the menu is open the browser hides the rest
     * of its window, address bar included, so the link is what the user means.
     */
    fun menuLink(): Pair<String, String>? {
        for (window in windows) {
            if (window.type != AccessibilityWindowInfo.TYPE_APPLICATION) continue
            val root = window.root ?: continue
            val pkg = root.packageName?.toString() ?: continue
            if (pkg !in BROWSER_URL_BARS) continue
            val url = root.findAccessibilityNodeInfosByViewId("$pkg:id/menu_header_url")
                .firstOrNull()?.text?.toString()?.let(::normalizeUrl) ?: continue
            val title = root.findAccessibilityNodeInfosByViewId("$pkg:id/menu_header_title")
                .firstOrNull()?.text?.toString().orEmpty()
            performGlobalAction(GLOBAL_ACTION_BACK)
            return url to title
        }
        return null
    }

    /**
     * Copies the text the user selected, by pressing Copy in the selection menu the app
     * shows. Apps rarely report a selection in web pages and text views, but every app
     * shows that menu, and Copy is labelled with the system's own word for it. Returns
     * whether Copy was pressed; the text is then on the clipboard.
     */
    fun copySelection(): Boolean {
        val labels = setOf(Resources.getSystem().getString(android.R.string.copy), "Copy")
        val screen = resources.displayMetrics.heightPixels
        val bounds = Rect()
        for (window in windows) {
            if (window.type != AccessibilityWindowInfo.TYPE_APPLICATION) continue
            // The menu is a window of its own, smaller than the app's. Looking only
            // there keeps a Copy button in the page itself from being pressed.
            window.getBoundsInScreen(bounds)
            if (bounds.height() > screen * 2 / 3) continue
            val root = window.root ?: continue
            if (root.packageName == packageName) continue
            // The menu is small, so walking it is cheap. (Searching by text misses
            // menu items that carry their label as a content description.)
            val queue = ArrayDeque<AccessibilityNodeInfo>().apply { add(root) }
            var seen = 0
            while (queue.isNotEmpty() && seen++ < MAX_MENU_NODES) {
                val node = queue.removeFirst()
                val name = (node.text ?: node.contentDescription)?.toString()?.trim()
                if (name != null && labels.any { it.equals(name, ignoreCase = true) }) {
                    var target: AccessibilityNodeInfo? = node
                    while (target != null && !target.isClickable) target = target.parent
                    if (target?.performAction(AccessibilityNodeInfo.ACTION_CLICK) == true) {
                        Log.d(TAG, "pressed Copy in ${root.packageName}")
                        return true
                    }
                }
                for (i in 0 until node.childCount) node.getChild(i)?.let(queue::add)
            }
        }
        return false
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
        private const val MAX_MENU_NODES = 200

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
