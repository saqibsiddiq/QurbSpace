'use client'

import { useEffect, useState } from 'react'
import { Icon } from '@/components/icon'
import { downloads } from '../site-config'
import { megabytes, type Release } from './release'

// The one button for the device the page is opened on (as the home page's
// GetIt finds it), with the version and size beside it; every device's file
// is in the cards below.

type Os = 'android' | 'linux' | 'windows' | 'mac' | 'ios' | 'other'

function detect(): Os {
  const ua = navigator.userAgent
  if (/Android/i.test(ua)) return 'android'
  if (/iPhone|iPad|iPod/i.test(ua) || (/Macintosh/i.test(ua) && navigator.maxTouchPoints > 1)) return 'ios'
  if (/Macintosh|Mac OS X/i.test(ua)) return 'mac'
  if (/Windows/i.test(ua)) return 'windows'
  if (/Linux|X11|CrOS/i.test(ua)) return 'linux'
  return 'other'
}

function Chips({ items }: { items: (string | null | undefined)[] }) {
  return (
    <ul className="dl-chips">
      {items.filter(Boolean).map((c) => (
        <li key={c}>{c}</li>
      ))}
    </ul>
  )
}

export function Hero({ release }: { release: Release | null }) {
  const [os, setOs] = useState<Os | null>(null)
  useEffect(() => {
    // ?as=android (or windows, linux, mac, ios) shows the page as that device
    // would, for looking at it.
    const as = new URLSearchParams(location.search).get('as') as Os | null
    setOs(as ?? detect())
  }, [])
  const v = release ? `Version ${release.version}` : null

  if (os === null) return <div className="dl-hero" aria-hidden />

  if (os === 'android')
    return (
      <div className="dl-hero">
        <a className="btn primary dl-big" href={downloads.android}>
          <Icon name="download" size={20} />
          Download for Android
        </a>
        <Chips items={['Android 8 or later', megabytes(release?.sizes.android), v]} />
      </div>
    )

  if (os === 'windows')
    return (
      <div className="dl-hero">
        <a className="btn primary dl-big" href={downloads.windows}>
          <Icon name="download" size={20} />
          Download for Windows
        </a>
        <Chips items={['Windows 10 or 11', megabytes(release?.sizes.windows), v]} />
      </div>
    )

  if (os === 'linux')
    return (
      <div className="dl-hero">
        <div className="dl-pair">
          <a className="btn primary dl-big" href={downloads.deb}>
            <Icon name="download" size={20} />
            Ubuntu and Debian
          </a>
          <a className="btn dl-big" href={downloads.appImage}>
            <Icon name="download" size={20} />
            Any Linux
          </a>
        </div>
        <Chips items={['Ubuntu, Debian, Mint, Fedora, Arch and more', v]} />
      </div>
    )

  if (os === 'mac' || os === 'ios')
    return (
      <div className="dl-hero">
        <a className="btn primary dl-big" href="/#waitlist">
          <Icon name="bell" size={20} />
          Tell me when it’s ready
        </a>
        <Chips items={[os === 'mac' ? 'Mac: not yet' : 'iPhone: not yet', 'Android, Windows and Linux below']} />
      </div>
    )

  return (
    <div className="dl-hero">
      <a className="btn primary dl-big" href="#all">
        <Icon name="download" size={20} />
        Choose your device
      </a>
      <Chips items={[v]} />
    </div>
  )
}
