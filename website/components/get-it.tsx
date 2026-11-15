'use client'

import { useEffect, useState, type FormEvent } from 'react'
import { Rise } from './rise'
import { downloads, repoUrl } from '../app/site-config'

// The last part of the landing page (docs/design/website-brief.md): getting
// it, kept plain, no drawing and none of the hero's words, at the owner's
// asking.
// One button that knows the visitor's device (Android and Linux download,
// Windows downloads marked early (built, never run on Windows: releasing.md),
// Mac and iPhone have no app and are offered the waitlist instead) then the
// waitlist itself and the code. Each button is the newest release's file
// itself (site-config's `downloads`), so nobody has to find it on GitHub.

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

const LINUX = [
  ['Ubuntu, Debian, Mint', downloads.deb],
  ['Any other Linux', downloads.appImage]
] as const

export function GetIt() {
  const [os, setOs] = useState<Os>('other')
  const [email, setEmail] = useState('')
  const [state, setState] = useState<'idle' | 'sending' | 'done' | 'closed' | 'error'>('idle')
  useEffect(() => {
    // While designing, ?gi=android (or linux, windows, mac, ios) shows this
    // scene alone as that device would see it.
    const as = process.env.NODE_ENV !== 'production' && new URLSearchParams(location.search).get('gi')
    if (as) document.documentElement.classList.add('scene-held', 'gi-held')
    setOs(as ? (as as Os) : detect())
  }, [])

  const waitlistFirst = os === 'mac' || os === 'ios'
  const forWhom = os === 'mac' ? 'Mac' : os === 'ios' ? 'iPhone' : null

  async function join(e: FormEvent) {
    e.preventDefault()
    setState('sending')
    try {
      const r = await fetch('/api/waitlist', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email }) })
      setState(r.ok ? 'done' : r.status === 503 ? 'closed' : 'error')
    } catch {
      setState('error')
    }
  }

  return (
    <section className="gi" id="get" aria-label="Get QurbSpace">
      <Rise lines={['Get QurbSpace.']} />

      {!waitlistFirst && (
        <div className="gi-get">
          {os === 'android' && (
            <a className="gi-btn" href={downloads.android}>Download for Android</a>
          )}
          {os === 'windows' && (
            <>
              <a className="gi-btn" href={downloads.windows}>Download for Windows</a>
              <small>Early: built, but not yet tested on Windows.</small>
            </>
          )}
          {os === 'linux' && (
            <div className="gi-linux">
              {LINUX.map(([who, href]) => (
                <a key={href} className="gi-btn gi-btn-small" href={href}>
                  {who}
                </a>
              ))}
            </div>
          )}
          {os === 'other' && <a className="gi-btn" href="/download">Download QurbSpace</a>}
          <a className="gi-more" href="/download">All downloads</a>
        </div>
      )}

      {waitlistFirst && (
        <p className="gi-note">
          {forWhom
            ? `QurbSpace for ${forWhom} isn’t ready yet. Join the waitlist and we’ll tell you when it is.`
            : 'Join the waitlist and we’ll tell you when there is something new.'}
        </p>
      )}

      <form className="gi-wait" id="waitlist" onSubmit={join}>
        {state === 'done' ? (
          <p className="gi-thanks">You’re on the list. We’ll write once, when it’s ready.</p>
        ) : (
          <>
            <label htmlFor="gi-email" className="gi-label">
              {waitlistFirst ? 'Join the waitlist' : 'Mac or iPhone? Join the waitlist'}
            </label>
            <div className="gi-field">
              <input id="gi-email" type="email" required maxLength={254} placeholder="you@example.com" value={email} onChange={(e) => setEmail(e.target.value)} />
              <button type="submit" disabled={state === 'sending'}>
                {state === 'sending' ? 'Joining…' : 'Join'}
              </button>
            </div>
            {state === 'closed' && <small className="gi-err">The waitlist isn’t open yet. Please try again soon.</small>}
            {state === 'error' && <small className="gi-err">That didn’t go through. Check the address and try again.</small>}
            <small className="gi-fine">Only your email, kept for this one message. Nothing else.</small>
          </>
        )}
      </form>

      <a className="gi-code" href={repoUrl}>See the code on GitHub →</a>
    </section>
  )
}
