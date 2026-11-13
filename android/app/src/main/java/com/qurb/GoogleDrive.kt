package com.qurb

import android.accounts.Account
import android.accounts.AccountManager
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * The person's Google Drive as the mailbox (`qurb_mailbox`, 2026-11-13): when
 * this phone cannot reach its computer, they leave each other files, sealed,
 * in QurbSpace's own hidden folder in that Drive.
 *
 * Signed in through Android's own accounts, with the Google account already
 * on the phone: no library added (the owner's choice, 2026-11-13). Allowed
 * `drive.appdata` -- QurbSpace's hidden folder, never the person's files. The
 * owner's Cloud project recognises the app by its package and signature
 * (docs/google-drive-setup.md). Before each sync the engine is given a token;
 * one Google refused is let go and another asked for.
 */
object GoogleDrive {
    private const val TAG = "QurbDrive"
    private const val PREFS = "drive"
    private const val ACCOUNT = "account"
    const val SCOPE = "oauth2:https://www.googleapis.com/auth/drive.appdata"
    private const val GOOGLE = "com.google"

    fun account(context: Context): String? =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(ACCOUNT, null)

    /** Android's own picker of the phone's Google accounts. */
    fun chooser(): Intent =
        AccountManager.newChooseAccountIntent(null, null, arrayOf(GOOGLE), null, null, null, null)

    /**
     * Ask the chosen account for QurbSpace's folder: Google shows its own
     * question the first time. On a yes the mailbox is on.
     */
    fun signIn(activity: Activity, name: String, then: (Boolean) -> Unit) {
        AccountManager.get(activity).getAuthToken(Account(name, GOOGLE), SCOPE, null, activity, { future ->
            val token = runCatching { future.result.getString(AccountManager.KEY_AUTHTOKEN) }
                .onFailure { Log.w(TAG, "Google Drive not allowed", it) }
                .getOrNull()
            if (token != null) {
                activity.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putString(ACCOUNT, name).apply()
                Engine.current()?.setDriveToken(token)
            }
            then(token != null)
        }, null)
    }

    fun signOut(context: Context) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().remove(ACCOUNT).apply()
        Engine.current()?.setDriveToken(null)
    }

    /**
     * A token for the engine before a sync, without showing anything: the
     * person allowed it once. One Google refused since is let go first.
     */
    suspend fun refresh(context: Context, engine: uniffi.qurb_mobile.Qurb) = withContext(Dispatchers.IO) {
        val name = account(context) ?: return@withContext
        val manager = AccountManager.get(context)
        val account = Account(name, GOOGLE)
        if (engine.driveTokenRefused()) {
            runCatching { manager.peekAuthToken(account, SCOPE)?.let { manager.invalidateAuthToken(GOOGLE, it) } }
        }
        val token = runCatching { manager.blockingGetAuthToken(account, SCOPE, true) }
            .onFailure { Log.w(TAG, "no Google Drive token", it) }
            .getOrNull()
        engine.setDriveToken(token)
    }
}
