package io.github.serhanensar.ec_android

import android.annotation.SuppressLint
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.PixelFormat
import android.graphics.Rect
import android.graphics.RectF
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.util.Log
import android.widget.Toast
import io.flutter.FlutterInjector
import io.flutter.embedding.android.FlutterTextureView
import io.flutter.embedding.android.FlutterView
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.embedding.engine.dart.DartExecutor
import io.flutter.plugin.common.MethodChannel
import kotlin.math.abs

/**
 * The fling handle, a thin strip on the left edge of the screen, and the full-screen
 * layer that a swipe in from it opens.
 *
 * Swiping in from the strip opens the layer, drawn by Flutter (`flingMain` in
 * lib/main.dart). The swipe goes on into the drag in one motion: Android keeps
 * delivering a touch to the window where it began, so the handle forwards the finger's
 * position to the layer. A swipe that lets go early leaves the card resting in the
 * layer, which then takes touches itself: the card is dragged there, a tap outside
 * closes. Taps and vertical swipes on the strip do nothing, so brushing the edge does
 * not open the layer by accident.
 *
 * Content shared to the app (ShareActivity.kt) opens in the same layer, with or without
 * the handle.
 */
class FlingHandle private constructor(private val context: Context) {
    private val windows = context.getSystemService(WindowManager::class.java)
    private val density = context.resources.displayMetrics.density
    private var handle: HandleView? = null
    private var layer: FlutterView? = null
    private var engine: FlutterEngine? = null
    private var channel: MethodChannel? = null
    // Whether the layer's Dart code listens yet, and what to show once it does.
    private var layerReady = false
    private var pending: Map<String, Any?>? = null

    /** Shows or hides the handle to match the user's choice and the permission. */
    fun sync() {
        val wanted = isEnabled(context) && Settings.canDrawOverlays(context)
        if (wanted && handle == null) show() else if (!wanted && handle != null) hide()
    }

    private fun show() {
        warmEngine()
        val view = HandleView(context)
        val params = WindowManager.LayoutParams(
            dp(HANDLE_WIDTH_DP), dp(160),
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT,
        ).apply { gravity = Gravity.START or Gravity.CENTER_VERTICAL }
        windows.addView(view, params)
        handle = view
    }

    private fun hide() {
        handle?.let { windows.removeView(it) }
        handle = null
        removeLayer()
    }

    /** Starts the Flutter engine for the layer early, so it appears without delay. */
    private fun warmEngine() {
        if (engine != null) return
        val loader = FlutterInjector.instance().flutterLoader()
        loader.startInitialization(context)
        loader.ensureInitializationComplete(context, null)
        val flutter = FlutterEngine(context)
        flutter.dartExecutor.executeDartEntrypoint(
            DartExecutor.DartEntrypoint(loader.findAppBundlePath(), "flingMain"),
        )
        channel = MethodChannel(flutter.dartExecutor.binaryMessenger, CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    // The layer tells which window it needs: "fling", "arrival" or "none".
                    // The layer starts listening; it shows what was shared while it started.
                    "takePending" -> {
                        layerReady = true
                        val content = pending
                        pending = null
                        if (content != null) showLayer()
                        result.success(content)
                    }
                    "layer" -> {
                        setLayer(call.arguments as String)
                        result.success(null)
                    }
                    "openFile" -> result.success(Files.open(context, call.arguments as String))
                    "showInFolder" -> result.success(Files.showFolder(context))
                    "openUrl" -> result.success(Files.openUrl(context, call.arguments as String))
                    "deviceName" -> result.success(DeviceInfo.name(context))
                    "receiveDir" -> result.success(DeviceInfo.receiveDir())
                    else -> result.notImplemented()
                }
            }
        }
        engine = flutter
    }

    private var layerMode = "none"

    /** Whether the fling layer's engine runs (it then shows arrivals itself). */
    fun isActive(): Boolean = engine != null

    private fun showLayer() = setLayer("fling")

    /** Opens the layer with shared [content] (see ShareActivity.kt), ready to drag. */
    fun share(content: Map<String, Any?>) {
        warmEngine()
        if (!layerReady) {
            pending = content
            return
        }
        showLayer()
        channel?.invokeMethod("start", content)
    }

    /**
     * Sizes the layer window for its job. "fling": the whole screen, taking touches
     * (a touch still going on the handle stays with the handle). "arrival": a strip at the top that can be
     * touched, sized to the arrival cards, so the rest of the screen keeps working.
     * "none": no window.
     */
    private fun setLayer(mode: String) {
        val flutter = engine ?: return
        if (mode == "none") {
            removeLayer()
            return
        }
        val params = if (mode == "fling") {
            WindowManager.LayoutParams(
                WindowManager.LayoutParams.MATCH_PARENT,
                WindowManager.LayoutParams.MATCH_PARENT,
                WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
                    WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
                PixelFormat.TRANSLUCENT,
            )
        } else {
            WindowManager.LayoutParams(
                dp(440), dp(ARRIVAL_HEIGHT_DP),
                WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL or
                    WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
                PixelFormat.TRANSLUCENT,
            ).apply { gravity = Gravity.TOP or Gravity.CENTER_HORIZONTAL }
        }
        val view = layer
        if (view == null) {
            val created = FlutterView(context, FlutterTextureView(context).apply { isOpaque = false })
            created.attachToFlutterEngine(flutter)
            windows.addView(created, params)
            layer = created
        } else if (mode != layerMode) {
            windows.updateViewLayout(view, params)
        }
        layerMode = mode
        flutter.lifecycleChannel.appIsResumed()
    }

    private fun removeLayer() {
        layer?.let {
            it.detachFromFlutterEngine()
            windows.removeView(it)
        }
        layer = null
        layerMode = "none"
        engine?.lifecycleChannel?.appIsInactive()
    }

    /**
     * Works out what to fling: text the user selected (or just copied), otherwise the
     * web page in the browser in front, otherwise a screenshot. For a page a screenshot
     * is taken too, as the card's picture. All need screen access; without it, explains
     * how to turn it on.
     */
    private fun resolveContent(copied: Boolean, done: (Map<String, Any?>?) -> Unit) {
        val access = ScreenAccessService.instance
        if (access == null) {
            Toast.makeText(
                context,
                "Turn on Ecosystem Connector screen access to fling what is on screen.",
                Toast.LENGTH_LONG,
            ).show()
            context.startActivity(
                Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
            return done(null)
        }
        val pressed = copied || access.copySelection()
        Log.d(TAG, "copied the selection: $pressed")
        if (pressed) return readCopied(access, done)
        // Some menus ignore an accessibility click; tap Copy instead.
        val copy = access.findCopy() ?: return resolveScreen(access, done)
        access.tap(copy) { tapped ->
            Log.d(TAG, "tapped Copy: $tapped")
            if (tapped) readCopied(access, done) else resolveScreen(access, done)
        }
    }

    /** The text just copied, or what is on screen if the clipboard has none. */
    private fun readCopied(access: ScreenAccessService, done: (Map<String, Any?>?) -> Unit) {
        // Give the app a moment to put the text on the clipboard.
        Handler(Looper.getMainLooper()).postDelayed({
            readClipboard { text ->
                if (text != null) {
                    done(mapOf("type" to "text", "text" to text.take(ShareActivity.MAX_TEXT_CHARS)))
                } else {
                    resolveScreen(access, done)
                }
            }
        }, 120)
    }

    /**
     * A link whose menu is open, otherwise the web page in the browser in front,
     * otherwise a screenshot.
     */
    private fun resolveScreen(access: ScreenAccessService, done: (Map<String, Any?>?) -> Unit) {
        val link = access.menuLink()
        val page = link ?: access.currentPage()
        // Keep the handle out of the screenshot. Fade it rather than hide it: a hidden
        // window loses the touch that is still in progress on it.
        val view = handle
        view?.alpha = 0f
        val capture = {
            access.capture { path ->
                view?.alpha = 1f
                when {
                    page != null -> done(
                        mapOf("type" to "page", "url" to page.first, "title" to page.second, "path" to path),
                    )
                    path != null -> done(mapOf("type" to "screenshot", "path" to path))
                    else -> {
                        Toast.makeText(context, "Could not take a screenshot.", Toast.LENGTH_SHORT).show()
                        done(null)
                    }
                }
            }
        }
        // After closing a link menu, wait for it to leave the screen.
        val delay = if (link != null) 250L else 48L
        if (view != null) view.postDelayed(capture, delay) else capture()
    }

    /**
     * Reads the clipboard. Android lets only the app with focus read it, so a tiny,
     * invisible window takes focus for a moment. Called only right after pressing Copy
     * for the user, never to look at what they copied before.
     */
    private fun readClipboard(done: (String?) -> Unit) {
        var finished = false
        lateinit var probe: View
        fun finish(text: String?) {
            if (finished) return
            finished = true
            runCatching { windows.removeView(probe) }
            done(text)
        }
        probe = object : View(context) {
            override fun onWindowFocusChanged(hasWindowFocus: Boolean) {
                super.onWindowFocusChanged(hasWindowFocus)
                if (!hasWindowFocus) return
                val clip = context.getSystemService(ClipboardManager::class.java).primaryClip
                val text = clip?.takeIf { it.itemCount > 0 }?.getItemAt(0)?.coerceToText(context)
                finish(text?.toString()?.trim()?.ifEmpty { null })
            }
        }
        val params = WindowManager.LayoutParams(
            1, 1,
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE or WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT,
        ).apply { gravity = Gravity.TOP or Gravity.START }
        windows.addView(probe, params)
        probe.postDelayed({ finish(null) }, 600)
    }

    private fun dp(value: Int) = (value * density).toInt()

    /** The visible pill and the touch area around it. */
    @SuppressLint("ViewConstructor")
    private inner class HandleView(context: Context) : View(context) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.argb(150, 255, 255, 255) }
        private val shadow = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.argb(60, 0, 0, 0) }
        // How far in the finger must swipe, mostly sideways, to open the layer.
        private val triggerDistance = 40 * density
        private val slop = 12 * density

        private var downX = 0f
        private var downY = 0f
        private var lastX = 0f
        private var lastY = 0f
        private var resolving = false
        private var released = false
        private var started = false
        // Whether Copy in a selection menu was pressed when the finger landed.
        private var copied = false
        // Set once the touch is a swipe in (it opened the layer) or clearly not one.
        private var decided = false

        // How far the swipe in has got, 0 to 1; the pill grows with it as feedback.
        private var progress = 0f
            set(value) {
                val clamped = value.coerceIn(0f, 1f)
                if (clamped != field) {
                    field = clamped
                    invalidate()
                }
            }

        override fun onLayout(changed: Boolean, l: Int, t: Int, r: Int, b: Int) {
            super.onLayout(changed, l, t, r, b)
            // Keep the system back gesture from taking touches that start on the handle.
            systemGestureExclusionRects = listOf(Rect(0, 0, width, height))
        }

        override fun onDraw(canvas: Canvas) {
            val w = (4 + 3 * progress) * density
            val h = (64 + 24 * progress) * density
            val left = (2 + 4 * progress) * density
            val top = (height - h) / 2
            paint.alpha = (140 + 100 * progress).toInt()
            canvas.drawRoundRect(RectF(left, top + density, left + w, top + h + density), w, w, shadow)
            canvas.drawRoundRect(RectF(left, top, left + w, top + h), w, w, paint)
        }

        @SuppressLint("ClickableViewAccessibility")
        override fun onTouchEvent(event: MotionEvent): Boolean {
            lastX = event.rawX
            lastY = event.rawY
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    downX = lastX
                    downY = lastY
                    resolving = false
                    released = false
                    started = false
                    decided = false
                    // Some apps hide their selection menu while the screen is touched,
                    // and a hidden menu cannot be pressed: copy the selection now.
                    copied = ScreenAccessService.instance?.copySelection() == true
                }
                MotionEvent.ACTION_MOVE -> when {
                    started -> channel?.invokeMethod("move", position())
                    !decided -> {
                        val dx = lastX - downX
                        val dy = abs(lastY - downY)
                        progress = dx / triggerDistance
                        if (dx >= triggerDistance && dx > 1.5f * dy) {
                            decided = true
                            progress = 0f
                            begin()
                        } else if (dy > slop && dy > dx) {
                            // A vertical swipe along the edge: not for us.
                            decided = true
                            progress = 0f
                        }
                    }
                }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                    released = true
                    progress = 0f
                    if (event.actionMasked == MotionEvent.ACTION_CANCEL) lastX = -1000f
                    if (started) channel?.invokeMethod("release", position())
                }
            }
            return true
        }

        private fun begin() {
            // Already flinging: the layer is in front and handles the touch.
            if (layerMode == "fling" || resolving) return
            Log.d(TAG, "swipe in")
            resolving = true
            performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
            resolveContent(copied) { content ->
                Log.d(TAG, "content: ${content?.get("type")}, released early: $released")
                resolving = false
                if (content == null) return@resolveContent
                showLayer()
                started = true
                // With the finger already up, the card just rests in the layer.
                channel?.invokeMethod("start", if (released) content else content + position())
            }
        }

        private fun position() = mapOf("x" to lastX.toDouble(), "y" to lastY.toDouble())
    }

    companion object {
        private const val TAG = "EcFling"
        private const val CHANNEL = "io.github.serhanensar.ecosystemconnector/fling"
        private const val ARRIVAL_HEIGHT_DP = 420
        // Wide enough to start a swipe on easily; the visible pill stays thin.
        private const val HANDLE_WIDTH_DP = 32
        private const val PREFS = "fling"
        private const val KEY_HANDLE = "handle"

        @Volatile private var instance: FlingHandle? = null

        fun get(context: Context): FlingHandle =
            instance ?: synchronized(this) {
                instance ?: FlingHandle(context.applicationContext).also { instance = it }
            }

        fun isEnabled(context: Context): Boolean =
            context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(KEY_HANDLE, false)

        fun setEnabled(context: Context, on: Boolean) {
            context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putBoolean(KEY_HANDLE, on).apply()
        }
    }
}
