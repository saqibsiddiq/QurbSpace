import { createHash, timingSafeEqual } from 'node:crypto'

// Who may see the dashboard: whoever brings DASHBOARD_KEY, once, and from
// then on the browser it was brought in, for a year (2026-11-22: "how do i
// open it without key? its inconvenient"). The browser keeps a fingerprint of
// the key, not the key, in a cookie only this site reads.

export const COOKIE = 'qs_dashboard'
export const YEAR = 60 * 60 * 24 * 365

const fingerprint = (key: string) => createHash('sha256').update(`qurbspace dashboard ${key}`).digest('hex')

function same(a: string, b: string) {
  const x = Buffer.from(a)
  const y = Buffer.from(b)
  return x.length === y.length && timingSafeEqual(x, y)
}

/** The key set, without what a paste can bring around it: a key pasted
 *  into Vercel's prompt with a line break after it matched nothing
 *  (2026-11-22). */
const wantedKey = () => process.env.DASHBOARD_KEY?.trim() || null

/** The key's fingerprint, for the cookie; none with no key set. */
export function remembered(): string | null {
  const wanted = wantedKey()
  return wanted ? fingerprint(wanted) : null
}

/** Whether this is the key. */
export function isKey(key: string | undefined): boolean {
  const wanted = wantedKey()
  return !!wanted && !!key && same(key.trim(), wanted)
}

/** Whether a cookie holds this browser as let in. */
export function isRemembered(cookie: string | undefined): boolean {
  const wanted = remembered()
  return !!wanted && !!cookie && same(cookie, wanted)
}
