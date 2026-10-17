package io.github.serhanensar.ec_android

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.PorterDuff
import android.graphics.PorterDuffXfermode
import android.graphics.RectF
import android.os.Bundle
import android.util.Size
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import android.widget.Button
import android.widget.FrameLayout
import android.widget.TextView
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.Camera
import androidx.camera.core.CameraSelector
import androidx.camera.core.FocusMeteringAction
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.core.content.ContextCompat
import com.google.zxing.BarcodeFormat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.LuminanceSource
import com.google.zxing.MultiFormatReader
import com.google.zxing.NotFoundException
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.common.GlobalHistogramBinarizer
import com.google.zxing.common.HybridBinarizer
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

/**
 * Full-screen camera that looks for a pairing QR code (an `ecpair:` link) and returns it.
 *
 * The screen is dimmed except for a square viewfinder; only that square is decoded, so
 * the code fills more of the image. Tapping focuses there, and the torch helps in dim
 * light. Decoding uses the open-source ZXing library; nothing leaves the device.
 */
class ScanActivity : ComponentActivity() {
    private val analysis = Executors.newSingleThreadExecutor()
    private val reader = MultiFormatReader().apply {
        setHints(
            mapOf(
                DecodeHintType.POSSIBLE_FORMATS to listOf(BarcodeFormat.QR_CODE),
                DecodeHintType.TRY_HARDER to true,
            ),
        )
    }
    @Volatile private var done = false

    private lateinit var preview: PreviewView
    private var camera: Camera? = null
    private var torchOn = false

    private val askCamera = registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if (granted) startCamera() else finish()
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val density = resources.displayMetrics.density
        preview = PreviewView(this).apply { scaleType = PreviewView.ScaleType.FILL_CENTER }
        val hint = TextView(this).apply {
            text = "Point the camera at the code on your other device"
            setTextColor(Color.WHITE)
            textSize = 16f
            gravity = Gravity.CENTER
        }
        val torch = Button(this).apply {
            text = "Light"
            setOnClickListener {
                torchOn = !torchOn
                camera?.cameraControl?.enableTorch(torchOn)
                text = if (torchOn) "Light off" else "Light"
            }
        }
        val close = Button(this).apply {
            text = "Cancel"
            setOnClickListener { finish() }
        }
        setContentView(FrameLayout(this).apply {
            setBackgroundColor(Color.BLACK)
            addView(preview)
            addView(Viewfinder(this@ScanActivity))
            addView(hint, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.CENTER,
            ).apply { topMargin = (220 * density).toInt() + viewfinderSidePx() / 2 })
            addView(torch, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT, FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL,
            ).apply { bottomMargin = (48 * density).toInt() })
            addView(close, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT, FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.TOP or Gravity.START,
            ).apply {
                topMargin = (32 * density).toInt()
                leftMargin = (16 * density).toInt()
            })
        })
        // Tap to focus where the finger is.
        preview.setOnTouchListener { view, event ->
            if (event.action == MotionEvent.ACTION_UP) {
                val point = preview.meteringPointFactory.createPoint(event.x, event.y)
                val action = FocusMeteringAction.Builder(point)
                    .setAutoCancelDuration(3, TimeUnit.SECONDS)
                    .build()
                camera?.cameraControl?.startFocusAndMetering(action)
                view.performClick()
            }
            true
        }
        if (checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            startCamera()
        } else {
            askCamera.launch(Manifest.permission.CAMERA)
        }
    }

    private fun viewfinderSidePx(): Int {
        val metrics = resources.displayMetrics
        return (minOf(metrics.widthPixels, metrics.heightPixels) * 0.62f).toInt()
    }

    private fun startCamera() {
        val future = ProcessCameraProvider.getInstance(this)
        future.addListener({
            val provider = future.get()
            val previewUseCase = Preview.Builder().build().also { it.surfaceProvider = preview.surfaceProvider }
            val analyzer = ImageAnalysis.Builder()
                .setResolutionSelector(
                    ResolutionSelector.Builder()
                        .setResolutionStrategy(
                            ResolutionStrategy(Size(1920, 1080), ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER),
                        )
                        .build(),
                )
                .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                .build()
                .also { it.setAnalyzer(analysis, ::decode) }
            provider.unbindAll()
            camera = provider.bindToLifecycle(this, CameraSelector.DEFAULT_BACK_CAMERA, previewUseCase, analyzer)
        }, ContextCompat.getMainExecutor(this))
    }

    private fun decode(image: ImageProxy) {
        image.use {
            if (done) return
            val plane = it.planes[0]
            val bytes = ByteArray(plane.buffer.remaining()).also { b -> plane.buffer.get(b) }
            // The viewfinder is centred, so its middle square is the same in any rotation.
            val side = (minOf(it.width, it.height) * 0.8f).toInt()
            val left = (it.width - side) / 2
            val top = (it.height - side) / 2
            val source = PlanarYUVLuminanceSource(
                bytes, plane.rowStride, it.height, left, top, side, side, false,
            )
            val text = tryDecode(source) ?: tryDecode(source.invert()) ?: return
            if (text.startsWith("ecpair:")) {
                done = true
                runOnUiThread {
                    window.decorView.performHapticFeedback(HapticFeedbackConstants.CONFIRM)
                    setResult(RESULT_OK, Intent().putExtra(EXTRA_INVITE, text))
                    finish()
                }
            }
        }
    }

    private fun tryDecode(source: LuminanceSource): String? {
        for (bitmap in listOf(BinaryBitmap(HybridBinarizer(source)), BinaryBitmap(GlobalHistogramBinarizer(source)))) {
            try {
                return reader.decodeWithState(bitmap).text
            } catch (_: NotFoundException) {
                // Try the next binarizer.
            } finally {
                reader.reset()
            }
        }
        return null
    }

    override fun onDestroy() {
        analysis.shutdown()
        super.onDestroy()
    }

    /** Dims everything except a rounded square, with accent corners. */
    private inner class Viewfinder(context: Context) : View(context) {
        private val dim = Paint().apply { color = Color.argb(150, 0, 0, 0) }
        private val clear = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            xfermode = PorterDuffXfermode(PorterDuff.Mode.CLEAR)
        }
        private val accent = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.rgb(0x2C, 0xC6, 0xD4)
            style = Paint.Style.STROKE
            strokeWidth = 5 * resources.displayMetrics.density
            strokeCap = Paint.Cap.ROUND
        }

        init {
            setLayerType(LAYER_TYPE_HARDWARE, null)
        }

        override fun onDraw(canvas: Canvas) {
            val side = viewfinderSidePx().toFloat()
            val box = RectF((width - side) / 2, (height - side) / 2, (width + side) / 2, (height + side) / 2)
            val radius = 28 * resources.displayMetrics.density
            canvas.drawRect(0f, 0f, width.toFloat(), height.toFloat(), dim)
            canvas.drawRoundRect(box, radius, radius, clear)
            val arm = side * 0.14f
            val corners = Path().apply {
                moveTo(box.left, box.top + arm); lineTo(box.left, box.top + radius / 2)
                quadTo(box.left, box.top, box.left + radius / 2, box.top); lineTo(box.left + arm, box.top)
                moveTo(box.right - arm, box.top); lineTo(box.right - radius / 2, box.top)
                quadTo(box.right, box.top, box.right, box.top + radius / 2); lineTo(box.right, box.top + arm)
                moveTo(box.right, box.bottom - arm); lineTo(box.right, box.bottom - radius / 2)
                quadTo(box.right, box.bottom, box.right - radius / 2, box.bottom); lineTo(box.right - arm, box.bottom)
                moveTo(box.left + arm, box.bottom); lineTo(box.left + radius / 2, box.bottom)
                quadTo(box.left, box.bottom, box.left, box.bottom - radius / 2); lineTo(box.left, box.bottom - arm)
            }
            canvas.drawPath(corners, accent)
        }
    }

    companion object {
        const val EXTRA_INVITE = "invite"
    }
}
