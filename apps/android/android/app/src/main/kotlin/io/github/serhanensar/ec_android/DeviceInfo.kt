package io.github.serhanensar.ec_android

import android.content.Context
import android.os.Build
import android.os.Environment
import android.provider.Settings
import java.io.File

/** Facts about this device that both Flutter engines (app and fling layer) ask for. */
object DeviceInfo {
    fun name(context: Context): String =
        Settings.Global.getString(context.contentResolver, "device_name")?.takeIf { it.isNotBlank() }
            ?: "${Build.MANUFACTURER} ${Build.MODEL}"

    /** Download/Ecosystem Connector, visible in the Files app. */
    fun receiveDir(): String {
        @Suppress("DEPRECATION")
        val downloads = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS)
        val dir = File(downloads, "Ecosystem Connector")
        dir.mkdirs()
        return dir.absolutePath
    }
}
