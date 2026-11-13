package com.qurb

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.PowerManager
import android.provider.Settings

/**
 * Whether Android lets the app run in the background, and asking it to.
 *
 * A phone that saves battery hard freezes an app nobody is looking at: on a
 * new OPPO a computer collecting its backup stood still while the app was
 * off screen (the owner, 2026-11-13). Asked once, on first opening -- an app
 * can ask nothing while it is being installed -- with the system's own
 * question, and again from Settings. Some makers keep a switch of their own
 * as well; once the system's is given, Settings opens the app's page there.
 */
object Battery {
    fun free(context: Context): Boolean =
        context.getSystemService(PowerManager::class.java)?.isIgnoringBatteryOptimizations(context.packageName) == true

    /** The system's question, or the app's page if it was answered already. */
    @SuppressLint("BatteryLife") // a sync app, sideloaded: what the exemption is for
    fun ask(context: Context) {
        val asking = if (free(context)) {
            Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:${context.packageName}"))
        } else {
            Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, Uri.parse("package:${context.packageName}"))
        }
        runCatching { context.startActivity(asking) }
            .onFailure { runCatching { context.startActivity(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)) } }
    }
}
