package com.qurb

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * *Stay connected* back at once after the phone starts and after the app
 * updates itself (decision 0078), rather than at the next background pass:
 * an update stops the app, and with it the listening (decision 0090).
 */
class StartAgain : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            Intent.ACTION_BOOT_COMPLETED, Intent.ACTION_MY_PACKAGE_REPLACED -> StayConnected.start(context)
        }
    }
}
