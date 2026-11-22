package com.qurb

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.net.wifi.SoftApConfiguration
import android.net.wifi.WifiManager
import android.net.wifi.p2p.WifiP2pConfig
import android.net.wifi.p2p.WifiP2pManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.coroutines.resume

/**
 * A Wi-Fi network straight between this phone and the computer, for a large
 * transfer (decision 0092): through the home router every byte crosses the
 * air twice, and the S23 and the laptop got 10 MB/s whatever carried it
 * (2026-11-19).
 *
 * Two ways, the person's choice in Advanced ([Controls.fastLink]):
 * - **Wi-Fi Direct**: a group this phone owns; the computer joins on its own
 *   Wi-Fi Direct interface and stays on its usual network.
 * - **Phone hotspot**: Android's local-only hotspot, made by the app with a
 *   name and password of its own, sharing no internet and changing no
 *   setting; the computer leaves its usual network while it lasts.
 *
 * The network's name and password go to the computer over QurbSpace's own
 * encrypted connection, to this person's own computer only, and the network
 * ends with the transfer. Android 13 and later: the Nearby Wi-Fi devices
 * permission, with no location asked.
 */
object DirectLink {
    private const val TAG = "QurbLink"

    /** The last link opened, to be closed. */
    private var hotspot: WifiManager.LocalOnlyHotspotReservation? = null
    private var p2p: Pair<WifiP2pManager, WifiP2pManager.Channel>? = null

    fun possible(): Boolean = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU

    fun allowed(context: Context): Boolean = possible() &&
        context.checkSelfPermission(Manifest.permission.NEARBY_WIFI_DEVICES) == PackageManager.PERMISSION_GRANTED

    /** Open the link the person chose, for at most [seconds]; null if none. */
    suspend fun open(context: Context, how: String, seconds: Long = 15): uniffi.qurb_mobile.DirectLink? {
        if (!allowed(context)) return null
        close()
        return withTimeoutOrNull(seconds * 1000) {
            runCatching {
                when (how) {
                    Controls.FAST_HOTSPOT -> openHotspot(context)
                    Controls.FAST_DIRECT -> openGroup(context)
                    else -> null
                }
            }.onFailure { Log.w(TAG, "no direct link", it) }.getOrNull()
        }.also { Log.i(TAG, if (it != null) "opened ${it.name} (${how})" else "no direct link ($how)") }
    }

    @android.annotation.SuppressLint("MissingPermission")
    private suspend fun openHotspot(context: Context): uniffi.qurb_mobile.DirectLink? {
        val wifi = context.applicationContext.getSystemService(WifiManager::class.java) ?: return null
        return suspendCancellableCoroutine { done ->
            wifi.startLocalOnlyHotspot(object : WifiManager.LocalOnlyHotspotCallback() {
                override fun onStarted(reservation: WifiManager.LocalOnlyHotspotReservation) {
                    hotspot = reservation
                    val config = reservation.softApConfiguration
                    @Suppress("DEPRECATION")
                    val name = config.ssid ?: run { reservation.close(); hotspot = null; done.resume(null); return }
                    val password = config.passphrase ?: run { reservation.close(); hotspot = null; done.resume(null); return }
                    val security = when (config.securityType) {
                        SoftApConfiguration.SECURITY_TYPE_WPA3_SAE -> "sae"
                        else -> "wpa-psk"
                    }
                    if (done.isActive) done.resume(uniffi.qurb_mobile.DirectLink(1u, name, password, security, ""))
                }

                override fun onFailed(reason: Int) {
                    Log.w(TAG, "the hotspot did not start: $reason")
                    if (done.isActive) done.resume(null)
                }
            }, Handler(Looper.getMainLooper()))
        }
    }

    @android.annotation.SuppressLint("MissingPermission")
    private suspend fun openGroup(context: Context): uniffi.qurb_mobile.DirectLink? {
        val manager = context.applicationContext.getSystemService(WifiP2pManager::class.java) ?: return null
        val channel = manager.initialize(context.applicationContext, Looper.getMainLooper(), null) ?: return null
        p2p = manager to channel
        // A group of an earlier run would be in the way.
        suspendCancellableCoroutine { done ->
            manager.removeGroup(channel, object : WifiP2pManager.ActionListener {
                override fun onSuccess() { if (done.isActive) done.resume(Unit) }
                override fun onFailure(reason: Int) { if (done.isActive) done.resume(Unit) }
            })
        }
        val random = java.security.SecureRandom()
        val letters = "abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789"
        fun word(n: Int) = (1..n).map { letters[random.nextInt(letters.length)] }.joinToString("")
        val name = "DIRECT-qb-${word(6)}"
        val password = word(20)
        val config = WifiP2pConfig.Builder()
            .setNetworkName(name)
            .setPassphrase(password)
            .setGroupOperatingBand(WifiP2pConfig.GROUP_OWNER_BAND_5GHZ)
            .enablePersistentMode(false)
            .build()
        val made = suspendCancellableCoroutine { done ->
            manager.createGroup(channel, config, object : WifiP2pManager.ActionListener {
                override fun onSuccess() { if (done.isActive) done.resume(true) }
                override fun onFailure(reason: Int) {
                    Log.w(TAG, "the Wi-Fi Direct group was not made: $reason")
                    if (done.isActive) done.resume(false)
                }
            })
        }
        if (!made) return null
        val address = suspendCancellableCoroutine<String?> { done ->
            manager.requestDeviceInfo(channel) { device -> if (done.isActive) done.resume(device?.deviceAddress) }
        } ?: return null
        return uniffi.qurb_mobile.DirectLink(2u, name, password, "wpa-psk", address)
    }

    /** End the link, if one is open. */
    fun close() {
        hotspot?.let { runCatching { it.close() } }
        hotspot = null
        p2p?.let { (manager, channel) ->
            runCatching { manager.removeGroup(channel, null) }
            runCatching { channel.close() }
        }
        p2p = null
    }
}
