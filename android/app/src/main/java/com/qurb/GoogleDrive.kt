package com.qurb

import android.content.Context
import android.util.Log
import com.google.android.gms.auth.api.identity.AuthorizationRequest
import com.google.android.gms.auth.api.identity.AuthorizationResult
import com.google.android.gms.auth.api.identity.Identity
import com.google.android.gms.common.api.Scope
import com.google.android.gms.tasks.Tasks
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.util.concurrent.TimeUnit

/**
 * The person's Google Drive as the mailbox (`qurb_mailbox`, decision 0088):
 * when this phone cannot reach its computer, they leave each other files,
 * sealed, in QurbSpace's own hidden folder in the phone's Drive.
 *
 * Through Google's sign-in (the owner's yes of 2026-11-14): its account sheet
 * rather than Android's old list, and with the Drive permission a one-time
 * code made for the computers, which this phone hands to each one it reaches
 * (`Request::DriveCode`). The computer exchanges it for a lasting sign-in to
 * this phone's account, so the person never signs in there: the phone may use
 * another Google account than the computer's, as the owner's new phone does.
 * Allowed `drive.appdata` -- QurbSpace's hidden folder, never the person's
 * files -- and the address, to show which account.
 */
object GoogleDrive {
    private const val TAG = "QurbDrive"
    private const val PREFS = "drive"
    private const val ACCOUNT = "account"
    private const val HANDED = "handed_at"
    private const val DRIVE = "https://www.googleapis.com/auth/drive.appdata"

    /** The web client the codes are made for (docs/google-drive-setup.md,
     *  step 8). An identifier, not a secret: it is in every copy of the app. */
    const val WEB_CLIENT_ID = "892093025334-8v20aphgklli5jjitd85ehh58chpg8jp.apps.googleusercontent.com"

    /** A code is handed again after this long: the computer keeps the
     *  sign-in, but a new computer, or one that lost it, gets it this way. */
    private const val HAND_AGAIN_MS = 24 * 60 * 60 * 1000L

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    fun account(context: Context): String? = prefs(context).getString(ACCOUNT, null)

    private fun request(code: Boolean, again: Boolean = false): AuthorizationRequest =
        AuthorizationRequest.builder()
            .setRequestedScopes(listOf(Scope(DRIVE), Scope("email")))
            .apply { if (code) requestOfflineAccess(WEB_CLIENT_ID, again) }
            .build()

    /**
     * Ask, with Google's own sheet: which account, and its yes for
     * QurbSpace's folder. `resolve` shows Google's screens when it needs to;
     * `then` hears whether the mailbox is on.
     */
    fun signIn(activity: MainActivity, then: (Boolean) -> Unit) {
        Identity.getAuthorizationClient(activity).authorize(request(code = true, again = true))
            .addOnSuccessListener { result ->
                val intent = result.pendingIntent
                if (result.hasResolution() && intent != null) {
                    activity.resolveDrive(intent) { answered -> then(answered != null && taken(activity, answered)) }
                } else {
                    then(taken(activity, result))
                }
            }
            .addOnFailureListener {
                Log.w(TAG, "Google Drive not allowed", it)
                then(false)
            }
    }

    /** What Google answered: the token and the code to the engine, the account kept. */
    fun taken(context: Context, result: AuthorizationResult): Boolean {
        val token = result.accessToken ?: return false
        val email = runCatching { result.toGoogleSignInAccount()?.email }.getOrNull()
        prefs(context).edit().putString(ACCOUNT, email ?: "Google account").apply()
        Engine.current()?.setDriveToken(token)
        result.serverAuthCode?.let { Engine.current()?.setDriveCode(it) }
        return true
    }

    fun signOut(context: Context) {
        prefs(context).edit().remove(ACCOUNT).remove(HANDED).apply()
        Engine.current()?.setDriveToken(null)
        Engine.current()?.setDriveCode(null)
    }

    /**
     * Before a sync, with nothing shown: a token for the engine, and once a
     * day a fresh code for the computers. Google keeps the yes the person
     * gave; if it wants them asked again, the mailbox waits for Settings.
     */
    suspend fun refresh(context: Context, engine: uniffi.qurb_mobile.Qurb) = withContext(Dispatchers.IO) {
        if (account(context) == null) return@withContext
        val now = System.currentTimeMillis()
        if (engine.driveCodeHanded()) prefs(context).edit().putLong(HANDED, now).apply()
        val code = now - prefs(context).getLong(HANDED, 0L) > HAND_AGAIN_MS
        val result = runCatching {
            Tasks.await(Identity.getAuthorizationClient(context).authorize(request(code)), 20, TimeUnit.SECONDS)
        }.onFailure { Log.w(TAG, "no Google Drive token", it) }.getOrNull() ?: return@withContext
        if (result.hasResolution()) return@withContext
        result.accessToken?.let { engine.setDriveToken(it) }
        result.serverAuthCode?.let { engine.setDriveCode(it) }
    }
}
