package io.github.serhanensar.ec_android

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.widget.Toast

/**
 * "Share → Ecosystem Connector" from any app. Has no screen of its own: what is shared
 * opens in the fling layer over the app it came from, as a card to drag onto a device.
 *
 * Shared files are copied into the cache first (the core reads plain files, and the
 * sharing app's permission to read them ends with this activity). A shared web link
 * becomes a page; other text becomes text. Without permission to draw over other apps,
 * the share goes to the app's main screen instead, as before.
 */
class ShareActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val shared = intent
        if (!Settings.canDrawOverlays(this)) {
            startActivity(
                Intent(shared)
                    .setClass(this, MainActivity::class.java)
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION),
            )
            return finish()
        }
        Thread {
            val content = contentOf(shared)
            runOnUiThread {
                if (content != null) {
                    FlingHandle.get(this).share(content)
                } else {
                    Toast.makeText(this, "Nothing that can be sent was shared.", Toast.LENGTH_SHORT).show()
                }
                finish()
            }
        }.start()
    }

    /** What to fling, in the form FlingHandle.kt passes to the layer; null if nothing usable. */
    private fun contentOf(shared: Intent): Map<String, Any?>? {
        val files = SharedFiles(this).copyShared(shared)
        if (files.isNotEmpty()) return mapOf("type" to "files", "paths" to files)
        val text = shared.getStringExtra(Intent.EXTRA_TEXT)?.trim()?.take(MAX_TEXT_CHARS)
        if (text.isNullOrEmpty()) return null
        val subject = shared.getStringExtra(Intent.EXTRA_SUBJECT)?.trim().orEmpty()
        val link = LINK.find(text)?.value
        // Apps often share "Title https://…": that is a page, the rest its title.
        return if (link != null && text.length - link.length <= MAX_TITLE_AROUND_LINK) {
            val title = subject.ifEmpty { text.replace(link, "").trim() }
            mapOf("type" to "page", "url" to link, "title" to title)
        } else {
            mapOf("type" to "text", "text" to text)
        }
    }

    companion object {
        private val LINK = Regex("""https?://\S+""", RegexOption.IGNORE_CASE)
        // Longer text around a link is a message that happens to contain one.
        private const val MAX_TITLE_AROUND_LINK = 200
        // Matches MAX_TEXT_CHARS in the ec-fling crate.
        const val MAX_TEXT_CHARS = 64 * 1024
    }
}
