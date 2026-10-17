package io.github.serhanensar.ec_android

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.provider.Settings
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.io.File

/**
 * Hosts the Flutter UI and answers its platform requests: device name, receive folder,
 * permissions and the background receive service, QR scanning and shared files.
 */
class MainActivity : FlutterActivity() {
    private var channel: MethodChannel? = null
    private var pendingScan: MethodChannel.Result? = null
    private var pendingPrepare: MethodChannel.Result? = null
    private val sharedFiles = SharedFiles(this)

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "deviceName" -> result.success(deviceName())
                    "receiveDir" -> result.success(receiveDir())
                    "prepare" -> prepare(result)
                    "scanInvite" -> scan(result)
                    "takeShared" -> result.success(sharedFiles.take())
                    else -> result.notImplemented()
                }
            }
        }
        sharedFiles.onReady = { paths -> runOnUiThread { channel?.invokeMethod("shared", paths) } }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        sharedFiles.handle(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        sharedFiles.handle(intent)
    }

    private fun deviceName(): String =
        Settings.Global.getString(contentResolver, "device_name")?.takeIf { it.isNotBlank() }
            ?: "${Build.MANUFACTURER} ${Build.MODEL}"

    /** Download/Ecosystem Connector, visible in the Files app. */
    private fun receiveDir(): String {
        @Suppress("DEPRECATION")
        val downloads = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS)
        val dir = File(downloads, "Ecosystem Connector")
        dir.mkdirs()
        return dir.absolutePath
    }

    /** Asks for the permissions the app needs, then starts the receive service. */
    private fun prepare(result: MethodChannel.Result) {
        val needed = buildList {
            if (Build.VERSION.SDK_INT >= 33) add(Manifest.permission.POST_NOTIFICATIONS)
            if (Build.VERSION.SDK_INT < 29) add(Manifest.permission.WRITE_EXTERNAL_STORAGE)
        }.filter { checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }

        if (needed.isEmpty()) {
            ReceiveService.start(this)
            result.success(null)
        } else {
            pendingPrepare = result
            requestPermissions(needed.toTypedArray(), REQUEST_PERMISSIONS)
        }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == REQUEST_PERMISSIONS) {
            // The app still works without notifications; start either way.
            ReceiveService.start(this)
            pendingPrepare?.success(null)
            pendingPrepare = null
        }
    }

    private fun scan(result: MethodChannel.Result) {
        pendingScan?.success(null)
        pendingScan = result
        @Suppress("DEPRECATION")
        startActivityForResult(Intent(this, ScanActivity::class.java), REQUEST_SCAN)
    }

    @Deprecated("Deprecated in Java")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        @Suppress("DEPRECATION")
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == REQUEST_SCAN) {
            pendingScan?.success(if (resultCode == RESULT_OK) data?.getStringExtra(ScanActivity.EXTRA_INVITE) else null)
            pendingScan = null
        }
    }

    companion object {
        const val CHANNEL = "io.github.serhanensar.ecosystemconnector/platform"
        private const val REQUEST_SCAN = 1
        private const val REQUEST_PERMISSIONS = 2
    }
}
