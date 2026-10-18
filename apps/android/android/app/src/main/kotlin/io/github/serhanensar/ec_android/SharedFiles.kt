package io.github.serhanensar.ec_android

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.OpenableColumns
import android.util.Log
import java.io.File

/**
 * Files other apps share to Ecosystem Connector arrive as content URIs. The Rust core
 * reads plain files, so each one is copied into the app's cache first.
 */
class SharedFiles(private val context: Context) {
    /** Called on a background thread when copied files are ready. */
    var onReady: ((List<String>) -> Unit)? = null

    private val ready = mutableListOf<String>()

    /** Returns and clears the files that are ready. */
    @Synchronized
    fun take(): List<String> = ready.toList().also { ready.clear() }

    fun handle(intent: Intent?) {
        val uris = urisOf(intent ?: return)
        if (uris.isEmpty()) return
        Thread {
            val paths = copyAll(uris)
            synchronized(this) { ready.addAll(paths) }
            onReady?.invoke(take())
        }.start()
    }

    /** Copies the files [intent] shares into the cache; blocks, so call it off the main thread. */
    fun copyShared(intent: Intent): List<String> = copyAll(urisOf(intent))

    private fun copyAll(uris: List<Uri>): List<String> {
        if (uris.isEmpty()) return emptyList()
        val dir = File(context.cacheDir, "shared/${System.currentTimeMillis()}").apply { mkdirs() }
        return uris.mapNotNull { copy(it, dir) }
    }

    private fun urisOf(intent: Intent): List<Uri> = when (intent.action) {
        Intent.ACTION_SEND -> listOfNotNull(stream(intent))
        Intent.ACTION_SEND_MULTIPLE -> streams(intent)
        else -> emptyList()
    }

    @Suppress("DEPRECATION")
    private fun stream(intent: Intent): Uri? =
        if (Build.VERSION.SDK_INT >= 33) intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
        else intent.getParcelableExtra(Intent.EXTRA_STREAM)

    @Suppress("DEPRECATION")
    private fun streams(intent: Intent): List<Uri> =
        (if (Build.VERSION.SDK_INT >= 33) intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
        else intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM)) ?: emptyList()

    private fun copy(uri: Uri, dir: File): String? = runCatching {
        val target = File(dir, safeName(displayName(uri)))
        context.contentResolver.openInputStream(uri)?.use { input ->
            target.outputStream().use { input.copyTo(it) }
        } ?: return null
        target.absolutePath
    }.onFailure { Log.w("EcShare", "could not read a shared file", it) }.getOrNull()

    private fun displayName(uri: Uri): String =
        context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
            ?.use { if (it.moveToFirst()) it.getString(0) else null }
            ?: uri.lastPathSegment
            ?: "shared-file"

    /** The name comes from another app; keep it to one plain path component. */
    private fun safeName(name: String): String =
        name.replace(Regex("[/\\\\:*?\"<>|\\p{Cntrl}]"), "_").trim().trimEnd('.').ifEmpty { "shared-file" }.take(200)
}
