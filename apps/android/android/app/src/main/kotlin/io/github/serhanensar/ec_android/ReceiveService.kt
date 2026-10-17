package io.github.serhanensar.ec_android

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat

/**
 * Keeps the process, and with it the Rust core, alive while the app is in the
 * background, so paired devices can still send files.
 *
 * It also holds a Wi-Fi multicast lock: without it Android drops the mDNS packets
 * devices use to find each other.
 */
class ReceiveService : Service() {
    private var multicastLock: WifiManager.MulticastLock? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL_ID, "Ready to receive", NotificationManager.IMPORTANCE_MIN).apply {
                description = "Shown while Ecosystem Connector can receive files in the background"
            },
        )
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_notify_sync)
            .setContentTitle("Ready to receive")
            .setContentText("Your paired devices can send files to this device.")
            .setContentIntent(open)
            .setOngoing(true)
            .build()
        ServiceCompat.startForeground(
            this,
            NOTIFICATION_ID,
            notification,
            if (Build.VERSION.SDK_INT >= 29) ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE else 0,
        )

        if (multicastLock == null) {
            val wifi = applicationContext.getSystemService(WifiManager::class.java)
            multicastLock = wifi.createMulticastLock("ecosystem-connector-mdns").apply {
                setReferenceCounted(false)
                acquire()
            }
        }
        FlingHandle.get(this).sync()
        // Not sticky: after the process dies the core is gone too, so a restarted
        // service would claim to be ready when it is not.
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        multicastLock?.release()
        multicastLock = null
        super.onDestroy()
    }

    companion object {
        private const val CHANNEL_ID = "presence"
        private const val NOTIFICATION_ID = 1

        fun start(context: Context) {
            context.startForegroundService(Intent(context, ReceiveService::class.java))
        }
    }
}
