# Setting up Google Drive for QurbSpace

The owner's part of the Google Drive mailbox (2026-11-13): a Google Cloud
project that lets QurbSpace use one hidden folder in a person's own Drive.
Done once, by the owner, for everyone who uses QurbSpace. Nothing here costs
money.

What QurbSpace will be allowed: **its own hidden app folder** in the Drive of
whoever signs in (the `drive.appdata` scope), and nothing else. It cannot see
or change that person's files, folders or anything else in their Drive. What it
puts there is encrypted on the device before it leaves.

Until the project is verified by Google (step 8), only Google accounts listed
as test users can sign in, up to 100. That is enough for testing.

---

## 1. Create the project

1. Open <https://console.cloud.google.com/> and sign in with the Google
   account you want to own QurbSpace's project.
2. Top bar → the project picker → **New project**.
3. Name: `QurbSpace`. Organisation: none is fine. **Create**.
4. Make sure the new project is selected in the top bar.

## 2. Turn on the Google Drive API

1. Left menu → **APIs & Services** → **Library**.
2. Search **Google Drive API** → open it → **Enable**.

## 3. The consent screen

What people see when they tap "Allow".

1. **APIs & Services** → **OAuth consent screen** (it may be called **Google
   Auth Platform** → **Branding**).
2. **Get started** / **Configure**:
   - App name: `QurbSpace`
   - User support email: your email
   - Audience: **External**
   - Contact email: your email
   - Agree to the policy → **Create**.
3. **Data access** (or **Scopes**) → **Add or remove scopes** → find and tick
   `https://www.googleapis.com/auth/drive.appdata`
   ("See, create, and delete its own configuration data in your Google
   Drive") → **Update** → **Save**. Add no other scope.
4. **Audience** (or **Test users**) → **Add users** → add your Google account,
   and the account on each phone you will test with (the new phone's included)
   → **Save**. Leave the publishing status on **Testing**.

## 4. The Android client: for the phone

1. **APIs & Services** → **Credentials** (or **Google Auth Platform** →
   **Clients**) → **Create credentials** → **OAuth client ID**.
2. Application type: **Android**.
3. Name: `QurbSpace Android`.
4. Package name: `com.qurb`
5. SHA-1 certificate fingerprint:
   `A7:46:6F:15:3A:6A:34:D6:B7:6E:53:54:2F:19:64:47:84:CD:E6:B2`
   (the release key's, read from the built APK with
   `apksigner verify --print-certs`; the same key signs the GitHub releases).
6. **Create**. Nothing to copy from it: the phone is recognised by its package
   and signature.

## 5. The desktop client: for the computer

1. **Create credentials** → **OAuth client ID** again.
2. Application type: **Desktop app**.
3. Name: `QurbSpace Desktop`.
4. **Create** → in the dialog, **Download JSON**.
5. Save the file as `~/.config/qurb/google-desktop-client.json` on the laptop.

The JSON holds a client ID and a "client secret". For a desktop app Google
does not treat that secret as secret (it ships inside every copy of the
program) but keep the file out of the repository anyway: the build will read
it from there, and it is not committed.

## 6. The web client: for the phone's account, handed to the computer

So the computer can use the phone's Google account without anyone signing in
on it (decision 0088, amended 2026-11-14): the phone asks Google for a
one-time code made for this client and hands it to the computer.

1. **Create credentials** → **OAuth client ID** again.
2. Application type: **Web application**.
3. Name: `QurbSpace Web`. No origins or redirect URIs.
4. **Create** → **Download JSON**.
5. Save it as `~/.config/qurb/google-web-client.json` on the laptop (mode
   600). Its client ID is also written in the phone app
   (`GoogleDrive.WEB_CLIENT_ID`); an identifier, not a secret.

## 7. Tell Claude it is done

Say the project is set up and the JSON is saved. No need to paste anything:
the build reads the file.

## 8. Later, before releasing to the public

While in **Testing**, only the test users of step 3 can sign in, and Google
asks them to sign in again every seven days. Before QurbSpace is offered to
everyone:

1. Publish the consent screen (**Audience** → **Publish app**).
2. Google may ask to verify the app: a home page, a privacy policy page, and a
   short video showing what the app does with the permission. `drive.appdata`
   is one of Drive's narrowest permissions, which keeps this to the lighter
   verification; check what the console asks for at that point.

Not needed for testing.
