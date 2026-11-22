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
        // On the channel the phone's own Wi-Fi is on, when that is 5 GHz: one
        // radio on two channels takes turns between them, and the group on
        // 5805 MHz beside the home network on 5765 carried 8.0 MB/s, one hop
        // or not (2026-11-22). Otherwise any 5 GHz channel.
        @Suppress("DEPRECATION")
        val home = context.applicationContext.getSystemService(WifiManager::class.java)?.connectionInfo?.frequency ?: 0
        val config = WifiP2pConfig.Builder()
            .setNetworkName(name)
            .setPassphrase(password)
            .apply {
                if (home in 4900..5900) setGroupOperatingFrequency(home)
                else setGroupOperatingBand(WifiP2pConfig.GROUP_OWNER_BAND_5GHZ)
            }
            .enablePersistentMode(false)
            .build()
        Log.i(TAG, "a group beside the home network on $home MHz")
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

    /** When a link last failed to bring the computer on: none tried for a
     *  while after, and nothing waits for one meanwhile. */
    @Volatile private var failedAt = 0L
    private const val AFTER_FAILING_MS = 10 * 60_000L

    /** Whether a large transfer should wait for a link now: the person's
     *  Fast link on, allowed, and none failed in the last ten minutes. */
    fun possibleNow(context: Context): Boolean =
        Controls.fastLink(context) != Controls.FAST_OFF && allowed(context) &&
            System.currentTimeMillis() - failedAt > AFTER_FAILING_MS

    /**
     * Make the link and bring the computer onto it (decision 0092, amended):
     * made, offered in a short pass, and waited for until the computer has
     * joined. The link if it did; none, and closed, if it did not in time:
     * then the transfer goes the usual way, never slower for having tried.
     */
    suspend fun establish(context: Context, engine: uniffi.qurb_mobile.Qurb): uniffi.qurb_mobile.DirectLink? {
        // Met over Wi-Fi Direct first: the computer stays on its usual
        // network, so it comes whoever is at it. Else the phone's own group,
        // which a computer joins only when nobody is at it.
        if (Controls.fastLink(context) == Controls.FAST_DIRECT) meet(context, engine)?.let { return it }
        val link = open(context, Controls.fastLink(context)) ?: return failed(engine)
        engine.setDirectLink(link)
        // A short pass hands the offer to the computer it reaches.
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            Engine.hearingTheNetwork(context) { runCatching { engine.syncWithin(10u) } }
        }
        // Said no at once by a computer somebody is using (decision 0092,
        // amended): closed, and the usual way, with no waiting for it.
        // Not a failure: none waits ten minutes for it (2026-11-22: a no at
        // 16:28 kept the link from two transfers with the laptop locked).
        if (!engine.directLinkTaken()) return failed(engine, declined = true)
        val joined = if (link.how == 2.toUByte()) waitForClient(25) else { kotlinx.coroutines.delay(8_000); true }
        if (!joined) return failed(engine)
        engine.setDirectLinkReady(true)
        Log.i(TAG, "the computer is on ${link.name}")
        return link
    }

    /**
     * Wi-Fi Direct made by the two together (decision 0092, the meeting):
     * this phone's Wi-Fi Direct address offered in a short pass, the
     * computer's back; then each connects to the other, this phone asking
     * to own the group. Made this way rather than by joining a group the
     * phone already owns, which NetworkManager does not do: it asks to make
     * one, and a phone owning one ignored it (2026-11-22).
     */
    @android.annotation.SuppressLint("MissingPermission")
    private suspend fun meet(context: Context, engine: uniffi.qurb_mobile.Qurb): uniffi.qurb_mobile.DirectLink? {
        if (!allowed(context)) return null
        close()
        val manager = context.applicationContext.getSystemService(WifiP2pManager::class.java) ?: return null
        val channel = manager.initialize(context.applicationContext, Looper.getMainLooper(), null) ?: return null
        p2p = manager to channel
        suspendCancellableCoroutine { done ->
            manager.removeGroup(channel, object : WifiP2pManager.ActionListener {
                override fun onSuccess() { if (done.isActive) done.resume(Unit) }
                override fun onFailure(reason: Int) { if (done.isActive) done.resume(Unit) }
            })
        }
        val me = suspendCancellableCoroutine<String?> { done ->
            manager.requestDeviceInfo(channel) { device -> if (done.isActive) done.resume(device?.deviceAddress) }
        } ?: return null.also { close() }
        val link = uniffi.qurb_mobile.DirectLink(3u, "", "", "", me)
        engine.setDirectLink(link)
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            Engine.hearingTheNetwork(context) { runCatching { engine.syncWithin(10u) } }
        }
        val computer = engine.directLinkPeer()
        if (computer == null || !engine.directLinkTaken()) {
            Log.i(TAG, "the computer does not meet over Wi-Fi Direct")
            engine.setDirectLink(null)
            close()
            return null
        }
        // Found first: Android connects only to a device it has seen.
        manager.discoverPeers(channel, null)
        var seen = false
        repeat(15) {
            if (!seen) {
                seen = suspendCancellableCoroutine { done ->
                    manager.requestPeers(channel) { peers ->
                        if (done.isActive) done.resume(peers.deviceList.any { it.deviceAddress.equals(computer, ignoreCase = true) })
                    }
                }
                if (!seen) kotlinx.coroutines.delay(1_000)
            }
        }
        if (!seen) return unmet(engine, "the computer was not seen")
        val config = WifiP2pConfig().apply {
            deviceAddress = computer
            wps.setup = android.net.wifi.WpsInfo.PBC
            groupOwnerIntent = WifiP2pConfig.GROUP_OWNER_INTENT_MAX
        }
        val asked = suspendCancellableCoroutine { done ->
            manager.connect(channel, config, object : WifiP2pManager.ActionListener {
                override fun onSuccess() { if (done.isActive) done.resume(true) }
                override fun onFailure(reason: Int) {
                    Log.w(TAG, "connecting to the computer failed: $reason")
                    if (done.isActive) done.resume(false)
                }
            })
        }
        if (!asked || !waitForClient(30)) return unmet(engine, "the computer did not come")
        engine.setDirectLinkReady(true)
        Log.i(TAG, "met the computer over Wi-Fi Direct")
        return link
    }

    private fun unmet(engine: uniffi.qurb_mobile.Qurb, why: String): uniffi.qurb_mobile.DirectLink? {
        Log.i(TAG, "not met: $why; the phone's own group")
        engine.setDirectLink(null)
        close()
        return null
    }

    private fun failed(engine: uniffi.qurb_mobile.Qurb, declined: Boolean = false): uniffi.qurb_mobile.DirectLink? {
        if (!declined) failedAt = System.currentTimeMillis()
        engine.setDirectLink(null)
        engine.setDirectPossible(false)
        close()
        Log.i(TAG, "no computer joined; the usual way")
        return null
    }

    @android.annotation.SuppressLint("MissingPermission")
    private suspend fun waitForClient(seconds: Int): Boolean {
        val (manager, channel) = p2p ?: return false
        repeat(seconds) {
            val joined = suspendCancellableCoroutine { done ->
                manager.requestGroupInfo(channel) { group -> if (done.isActive) done.resume(group?.clientList?.isNotEmpty() == true) }
            }
            if (joined) return true
            kotlinx.coroutines.delay(1_000)
        }
        return false
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
