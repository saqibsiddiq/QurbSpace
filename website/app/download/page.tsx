import type { Metadata } from 'next'
import Link from 'next/link'
import { Icon } from '@/components/icon'
import { SiteFooter } from '@/components/site-footer'
import { SiteHeader } from '@/components/site-header'
import { downloads, repoDoc } from '../site-config'
import { Hero } from './hero'
import { latestRelease, megabytes } from './release'
import './download.css'

export const metadata: Metadata = {
  title: 'Get QurbSpace',
  description: 'Download QurbSpace for Android, Windows and Linux: the newest beta, straight from here.'
}

// The download page (the owner, 2026-11-23: "can u make the download page
// better?"): the button for the visitor's device first, then every device's
// file with its version and size, what to do if the system warns, and the
// three steps after. As few words as will do.
export const revalidate = 3600

type Card = {
  icon: 'smartphone' | 'laptop' | 'monitor'
  name: string
  needs: string
  files: { label: string; href: string; size?: number; note?: string }[]
  badge?: string
}

export default async function DownloadPage() {
  const release = await latestRelease()
  const cards: Card[] = [
    {
      icon: 'smartphone',
      name: 'Android',
      needs: 'Android 8 or later',
      files: [{ label: 'Download', href: downloads.android, size: release?.sizes.android, note: 'updates itself' }]
    },
    {
      icon: 'laptop',
      name: 'Windows',
      needs: 'Windows 10 or 11',
      badge: 'Early',
      files: [{ label: 'Download', href: downloads.windows, size: release?.sizes.windows, note: 'updates itself' }]
    },
    {
      icon: 'monitor',
      name: 'Linux',
      needs: 'Ubuntu, Debian, Mint, Fedora, Arch and more',
      files: [
        { label: 'Ubuntu and Debian', href: downloads.deb, size: release?.sizes.deb, note: '.deb' },
        { label: 'Any Linux', href: downloads.appImage, size: release?.sizes.appImage, note: 'AppImage: allow it to run, then open it' }
      ]
    }
  ]

  return (
    <div className="page">
      <SiteHeader />
      <main className="wrap dl">
        <section className="dl-top glass-frosted">
          <Link href="/" className="dl-back">
            <Icon name="arrow-right" size={16} />
            QurbSpace
          </Link>
          <span className="dl-beta">Open beta</span>
          <h1>Get QurbSpace</h1>
          <p className="dl-intro">For your phone and your computer. Install it on both.</p>
          <Hero release={release} />
        </section>

        <section id="all" className="dl-section">
          <h2>Every device</h2>
          <div className="dl-cards">
            {cards.map((c) => (
              <article key={c.name} className="dl-card glass-frosted">
                <header>
                  <span className="dl-tile">
                    <Icon name={c.icon} size={22} />
                  </span>
                  <div>
                    <h3>
                      {c.name}
                      {c.badge && <span className="dl-badge">{c.badge}</span>}
                    </h3>
                    <p>{c.needs}</p>
                  </div>
                </header>
                <div className="dl-files">
                  {c.files.map((f) => (
                    <a key={f.href} className="dl-file" href={f.href}>
                      <Icon name="download" size={18} />
                      <span className="dl-file-name">{f.label}</span>
                      <span className="dl-file-meta">{[megabytes(f.size), f.note].filter(Boolean).join(', ')}</span>
                    </a>
                  ))}
                </div>
              </article>
            ))}
            <article className="dl-card dl-card-soon glass-frosted">
              <header>
                <span className="dl-tile">
                  <Icon name="clock" size={22} />
                </span>
                <div>
                  <h3>Mac and iPhone</h3>
                  <p>Not ready yet</p>
                </div>
              </header>
              <div className="dl-files">
                <a className="dl-file" href="/#waitlist">
                  <Icon name="bell" size={18} />
                  <span className="dl-file-name">Join the waitlist</span>
                  <span className="dl-file-meta">One email, when it’s ready</span>
                </a>
              </div>
            </article>
          </div>
          {release && (
            <p className="dl-fine">
              Version {release.version}, released{' '}
              {new Date(release.published).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' })}.
              Built from the <a href="https://github.com/saqibsiddiq/QurbSpace">open code</a> on GitHub.
            </p>
          )}
        </section>

        <section className="dl-section">
          <h2>Then</h2>
          <ol className="dl-steps">
            <li className="glass-frosted">
              <span className="dl-num">1</span>
              <Icon name="laptop" size={22} />
              <h3>Open it on the computer</h3>
              <p>
                Go to <b>Devices</b>, then <b>Add a device</b>.
              </p>
            </li>
            <li className="glass-frosted">
              <span className="dl-num">2</span>
              <Icon name="qr-code" size={22} />
              <h3>Join from the phone</h3>
              <p>
                Scan the <b>QR code</b> or type the <b>six digits</b>. Approve on the computer.
              </p>
            </li>
            <li className="glass-frosted">
              <span className="dl-num">3</span>
              <Icon name="check" size={22} />
              <h3>That’s it</h3>
              <p>They find each other at home and away. Nothing else to set up.</p>
            </li>
          </ol>
        </section>

        <section className="dl-section">
          <h2>If it warns you</h2>
          <p className="dl-lead">Expected for a new app from outside an app store.</p>
          <div className="dl-warns">
            <div className="dl-warn glass-frosted">
              <h3>
                <Icon name="smartphone" size={18} />
                Android
              </h3>
              <p className="dl-quote">This app may be harmful</p>
              <p className="dl-taps">
                <span>More details</span>
                <Icon name="chevron-right" size={16} />
                <span>Install anyway</span>
              </p>
              <p className="dl-note">
                On Samsung, turn off <b>Auto Blocker</b> while installing: Settings, Security and privacy.
              </p>
            </div>
            <div className="dl-warn glass-frosted">
              <h3>
                <Icon name="laptop" size={18} />
                Windows
              </h3>
              <p className="dl-quote">Windows protected your PC</p>
              <p className="dl-taps">
                <span>More info</span>
                <Icon name="chevron-right" size={16} />
                <span>Run anyway</span>
              </p>
              <p className="dl-note">The installer isn’t signed yet.</p>
            </div>
          </div>
        </section>

        <section className="dl-section dl-last">
          <p className="callout">
            <Icon name="info" size={20} />
            <span>
              <b>A beta.</b> Keep your own copy of anything you can’t lose. See{' '}
              <a href={repoDoc('README.md#known-gaps')}>what isn’t finished yet</a>, or{' '}
              <Link href="/docs">read the guide</Link>.
            </span>
          </p>
          <p className="dl-fine">
            Building it yourself? <a href={repoDoc('docs/trying-it.md')}>From the source</a> and{' '}
            <a href={repoDoc('android/README.md')}>the Android app</a>.
          </p>
        </section>
      </main>
      <SiteFooter />
    </div>
  )
}
