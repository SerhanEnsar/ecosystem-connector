package io.github.serhanensar.ec_android

import android.app.DownloadManager
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import java.io.File

/** Opening received files, their folder and flung pages from the arrival card. */
object Files {
    /** Opens a received file with the app the user chose for its type. */
    fun open(context: Context, path: String): Boolean {
        val file = File(path)
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", file)
        val type = MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase())
            ?: "application/octet-stream"
        val intent = Intent(Intent.ACTION_VIEW).setDataAndType(uri, type)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        return start(context, Intent.createChooser(intent, null).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    /**
     * Opens the folder of [path] (the receive folder when there is none) in a file
     * manager: Samsung My Files at the folder, else the system Files app at it, else
     * the downloads list.
     */
    fun showFolder(context: Context, path: String? = null): Boolean {
        val dir = path?.let { File(it).parentFile?.path } ?: DeviceInfo.receiveDir()
        val samsung = Intent("samsung.myfiles.intent.action.LAUNCH_MY_FILES")
            .setPackage("com.sec.android.app.myfiles")
            .putExtra("samsung.myfiles.intent.extra.START_PATH", dir)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        // Not our URI, so no permission flags: the Files app has its own access. Its
        // document ids are paths on the primary volume.
        val storage = android.os.Environment.getExternalStorageDirectory().path
        val relative = dir.removePrefix(storage).trimStart('/').ifEmpty { "Download/ECOnnector" }
        val folder = DocumentsContract.buildDocumentUri(
            "com.android.externalstorage.documents", "primary:$relative",
        )
        val files = Intent(Intent.ACTION_VIEW)
            .setDataAndType(folder, DocumentsContract.Document.MIME_TYPE_DIR)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        val downloads = Intent(DownloadManager.ACTION_VIEW_DOWNLOADS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        return start(context, samsung) || start(context, files) || start(context, downloads)
    }

    /** Opens a flung page (already checked to be http or https) in the browser. */
    fun openUrl(context: Context, url: String): Boolean {
        val uri = Uri.parse(url)
        if (uri.scheme != "http" && uri.scheme != "https") return false
        return start(context, Intent(Intent.ACTION_VIEW, uri).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    private fun start(context: Context, intent: Intent): Boolean = try {
        context.startActivity(intent)
        true
    } catch (_: ActivityNotFoundException) {
        false
    } catch (_: SecurityException) {
        false
    }
}
