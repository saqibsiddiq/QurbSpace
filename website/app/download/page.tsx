import type { Metadata } from 'next'
import Link from 'next/link'
import { DocPage, LinkRow } from '@/components/doc-page'
import { Icon } from '@/components/icon'
import { downloads, repoDoc } from '../site-config'

export const metadata: Metadata = {
  title: 'Get QurbSpace',
  description: 'Download QurbSpace for Android, Windows and Linux: the newest beta, straight from here.'
}

export default function DownloadPage() {
  return (
    <DocPage
      title="Get QurbSpace"
      intro="The open beta, for your phone and your computer. Pick yours; the download starts at once."
    >
      <p className="callout">
        <Icon name="circle-check" size={20} />
        <span>
          <b>A beta.</b> It works every day between an Android phone and a Linux computer. Keep your own copy of
          anything you can’t lose, and read <a href={repoDoc('README.md#known-gaps')}>what isn’t finished yet</a>.
        </span>
      </p>

      <h2>Your phone</h2>
      <div className="rows">
        <LinkRow
          href={downloads.android}
          icon="smartphone"
          title="QurbSpace for Android"
          text="Android 8 or later. Open the file and allow installing when asked. It updates itself."
        />
      </div>

      <h2>Your computer</h2>
      <div className="rows">
        <LinkRow
          href={downloads.windows}
          icon="laptop"
          title="QurbSpace for Windows"
          text="Windows 10 or 11. Run the installer. Not yet tested by us: tell us how it goes."
        />
        <LinkRow
          href={downloads.deb}
          icon="laptop"
          title="QurbSpace for Ubuntu and Debian"
          text="Also Mint and Pop!_OS. Open the file to install it. It doesn’t update itself yet."
        />
        <LinkRow
          href={downloads.appImage}
          icon="laptop"
          title="QurbSpace for any Linux"
          text="Fedora, Arch and the rest. Allow it to run (Properties, Permissions), then open it. It updates itself."
        />
      </div>
      <p>Not sure which Linux you have? The last one works on all of them. Mac and iPhone aren’t ready yet.</p>

      <h2>If it warns you</h2>
      <p>
        Expected: QurbSpace is new and comes from here, not an app store. The code is all on GitHub, and each release is
        built from it there.
      </p>
      <ul>
        <li>
          <strong>Android</strong>: Play Protect may say <em>This app may be harmful</em>. Tap{' '}
          <strong>More details</strong>, then <strong>Install anyway</strong>. It warns because the app installs its own
          updates. On a Samsung phone, turn off <em>Auto Blocker</em> (Settings, Security and privacy) while you install.
        </li>
        <li>
          <strong>Windows</strong>: <em>Windows protected your PC</em>. Tap <strong>More info</strong>, then{' '}
          <strong>Run anyway</strong>. The installer isn’t signed yet.
        </li>
      </ul>

      <h2>Then</h2>
      <ol>
        <li>
          Open QurbSpace on the computer. It sets itself up; go to <strong>Devices</strong>, then{' '}
          <strong>Add a device</strong>.
        </li>
        <li>
          Open QurbSpace on the phone and join with the <strong>QR code</strong> or the <strong>six digits</strong> the
          computer shows. Check both screens show the same number, then approve on the computer.
        </li>
        <li>
          On the phone, tap <strong>Tap to connect</strong>. While it says <em>Connected</em>, anything the computer
          sends arrives in seconds. Tap the notification to disconnect.
        </li>
      </ol>
      <p>
        At home they talk straight across your WiFi. Away from home they still find each other, with nothing for you to
        set up. More in <Link href="/docs">the guide</Link>.
      </p>

      <h2>Building it yourself</h2>
      <div className="rows">
        <LinkRow
          href={repoDoc('docs/trying-it.md')}
          icon="file-text"
          title="From the source"
          text="Every step, from one computer to a phone"
        />
        <LinkRow
          href={repoDoc('android/README.md')}
          icon="smartphone"
          title="The Android app"
          text="Building it, signing a release, and what it does in the background"
        />
      </div>
    </DocPage>
  )
}
