package com.qurb

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat

/**
 * *Stay connected* (the owner, 2026-11-14): the app kept awake in the
 * background, listening for the person's computers saying they have something
 * for it, so a file sent to the phone arrives in seconds rather than at the
 * next pass. Without push (decision 0082), Android freezes a backgrounded app
 * and nothing could tell it; this is the way Syncthing and Tailscale take.
 *
 * Its notification is the least Android allows: its own channel at the
 * lowest importance (no sound, no vibration, no icon in the status bar on
 * most phones, folded at the foot of the shade) one word, in the app's
 * green. Connected by the person with Home's Sync, and disconnected by
 * tapping the notification, which goes with it (the owner, 2026-11-14:
 * "users can just click on the notification incase they want to
 * disconnect"). A phone connected stays so through a restart or an update.
 */
class StayConnected : Service() {
    @Volatile private var running = false

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == DISCONNECT) {
            Controls.setStayConnected(this, false)
            running = false
            ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_REMOVE)
            stopSelf()
            return START_NOT_STICKY
        }
        val type = if (Build.VERSION.SDK_INT >= 34) ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE else 0
        runCatching { ServiceCompat.startForeground(this, ID, notification(this), type) }
            .onFailure { Log.w(TAG, "could not stay connected", it); stopSelf(); return START_NOT_STICKY }
        if (!running) {
            running = true
            Thread(::listen, "qurb-stay-connected").apply { isDaemon = true }.start()
        }
        return START_STICKY
    }

    override fun onDestroy() {
        running = false
        super.onDestroy()
    }

    /** Wait for news, sync when it comes, and wait again. */
    private fun listen() {
        while (running && Controls.stayConnected(this) && Engine.isSetUp(this)) {
            val heard = runCatching { Engine.openNow(this).waitForNews(WAIT_SECONDS) }
                .onFailure { Log.w(TAG, "listening failed", it) }
            when {
                heard.isFailure -> Thread.sleep(30_000)
                heard.getOrDefault(false) -> {
                    Log.i(TAG, "a computer has something for this phone; syncing")
                    SyncWorker.runNow(this)
                    // A computer says it on every round while something
                    // waits: one pass for it, not one for each saying.
                    Thread.sleep(10_000)
                }
            }
        }
        running = false
        stopSelf()
    }

    companion object {
        private const val TAG = "QurbConnected"
        private const val DISCONNECT = "com.qurb.DISCONNECT"
        private const val CHANNEL = "connected"
        private const val ID = 7301
        /** How long one wait listens before starting afresh: a phone that
         *  moved networks listens from where it is now. */
        private const val WAIT_SECONDS = 600u

        /** Started where it should run and is not: when the app opens, and
         *  after each background pass, which is how it comes back after the
         *  phone restarts. Best effort: Android may refuse a start from the
         *  background, and the next pass tries again. */
        fun start(context: Context) {
            if (!Controls.stayConnected(context) || !Engine.isSetUp(context)) return
            runCatching { context.startForegroundService(Intent(context, StayConnected::class.java)) }
                .onFailure { Log.i(TAG, "not started now", it) }
        }

        /** Home's Sync: connected from now until the notification is tapped. */
        fun connect(context: Context) {
            Controls.setStayConnected(context, true)
            start(context)
        }

        private fun notification(context: Context): Notification {
            val manager = context.getSystemService(NotificationManager::class.java)
            if (manager?.getNotificationChannel(CHANNEL) == null) {
                manager?.createNotificationChannel(
                    NotificationChannel(CHANNEL, "Stay connected", NotificationManager.IMPORTANCE_MIN).apply {
                        description = "Shown while QurbSpace listens for your computers. Silent."
                        setShowBadge(false)
                        setSound(null, null)
                        enableVibration(false)
                    }
                )
            }
            // A tap disconnects.
            val disconnect = PendingIntent.getService(
                context, 0,
                Intent(context, StayConnected::class.java).setAction(DISCONNECT),
                PendingIntent.FLAG_IMMUTABLE,
            )
            return NotificationCompat.Builder(context, CHANNEL)
                .setSmallIcon(R.drawable.qurb_glyph)
                .setColor(context.getColor(R.color.green))
                .setContentTitle("Connected")
                .setContentText("Tap to disconnect")
                .setContentIntent(disconnect)
                .setOngoing(true)
                .setSilent(true)
                .setShowWhen(false)
                .setPriority(NotificationCompat.PRIORITY_MIN)
                .setCategory(NotificationCompat.CATEGORY_SERVICE)
                .setVisibility(NotificationCompat.VISIBILITY_SECRET)
                .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
                .build()
        }
    }
}
