import type { Metadata } from 'next'
import { cookies } from 'next/headers'
import { notFound, redirect } from 'next/navigation'
import { COOKIE, isKey, isRemembered } from './access'
import { load, type Platform } from './data'
import './dashboard.css'

// The owner's view of how QurbSpace is doing (2026-11-22: "a dashboard where
// i can monitor information like downloads"). Opened with the key set in
// Vercel as DASHBOARD_KEY (/dashboard?key=...) once per browser, which is
// then remembered for a year (access.ts); without either, or with no key set,
// there is no page here. Never indexed.
export const dynamic = 'force-dynamic'

export const metadata: Metadata = {
  title: 'Dashboard',
  robots: { index: false, follow: false }
}

const INSTALLS: Platform[] = ['Android', 'Windows', 'Ubuntu and Debian', 'Any Linux']
const UPDATES: Platform[] = ['Phone updates', 'Computer updates', 'Phone update checks', 'Computer update checks']

const number = (n: number | undefined | null) => (n ?? 0).toLocaleString('en')
const date = (iso: string) => new Date(iso).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' })

function Bars({ rows }: { rows: { label: string; value: number }[] }) {
  const top = Math.max(1, ...rows.map((r) => r.value))
  return (
    <div className="db-bars">
      {rows.map((r) => (
        <div className="db-bar" key={r.label}>
          <span className="db-bar-label">{r.label}</span>
          <span className="db-bar-track">
            <span className="db-bar-fill" style={{ width: `${(r.value / top) * 100}%` }} />
          </span>
          <span className="db-bar-value">{number(r.value)}</span>
        </div>
      ))}
    </div>
  )
}

export default async function DashboardPage({ searchParams }: { searchParams: Promise<{ key?: string }> }) {
  const { key } = await searchParams
  // Brought the key: remembered, and the address cleaned of it.
  if (isKey(key)) redirect(`/dashboard/enter?key=${encodeURIComponent(key!)}`)
  if (!isRemembered((await cookies()).get(COOKIE)?.value)) notFound()

  const d = await load()
  const installs = INSTALLS.reduce((n, p) => n + (d.totals[p] ?? 0), 0)

  return (
    <main className="db">
      <header className="db-head">
        <h1>Dashboard</h1>
        <p>As GitHub counts it, {date(d.at)}. Nothing here comes from anybody’s devices.</p>
      </header>

      <section className="db-tiles">
        <div className="db-tile">
          <b>{number(installs)}</b>
          <span>downloads</span>
        </div>
        <div className="db-tile">
          <b>{number(d.totals['Phone update checks'])}</b>
          <span>phone update checks</span>
        </div>
        <div className="db-tile">
          <b>{number(d.totals['Computer update checks'])}</b>
          <span>computer update checks</span>
        </div>
        <div className="db-tile">
          <b>{d.waitlist === null ? 'not set' : number(d.waitlist)}</b>
          <span>on the Mac and iPhone waitlist</span>
        </div>
        {d.repo && (
          <div className="db-tile">
            <b>{number(d.repo.stars)}</b>
            <span>stars, {number(d.repo.issues)} open issues</span>
          </div>
        )}
      </section>

      <section>
        <h2>Downloads by device</h2>
        <Bars rows={INSTALLS.map((p) => ({ label: p, value: d.totals[p] ?? 0 }))} />
      </section>

      <section>
        <h2>By release</h2>
        <div className="db-table">
          <table>
            <thead>
              <tr>
                <th>Release</th>
                <th>Published</th>
                <th>Tests</th>
                {[...INSTALLS, ...UPDATES].map((p) => (
                  <th key={p}>{p}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {d.releases.map((r) => (
                <tr key={r.tag}>
                  <td>{r.tag}</td>
                  <td>{date(r.published)}</td>
                  <td>{number(r.tests)}</td>
                  {[...INSTALLS, ...UPDATES].map((p) => (
                    <td key={p}>{number(r.byPlatform[p])}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="db-note">
          Downloads are the files people are sent to. Left out: {number(d.tests)} downloads by QurbSpace’s own platform
          tests (Tests), and the files the apps update themselves from (Phone updates, Computer updates). Update checks
          are each installed app asking whether there is a newer version: a rough measure of how many copies are in use,
          not of people. A download may be the same person twice.
        </p>
      </section>

      {d.traffic && (
        <section>
          <h2>Visits to GitHub, the last 14 days</h2>
          <div className="db-tiles">
            <div className="db-tile">
              <b>{number(d.traffic.views.uniques)}</b>
              <span>visitors, {number(d.traffic.views.count)} views</span>
            </div>
            <div className="db-tile">
              <b>{number(d.traffic.clones.uniques)}</b>
              <span>cloned the code</span>
            </div>
          </div>
          <Bars rows={d.traffic.days.map((v) => ({ label: v.day.slice(5), value: v.uniques }))} />
          <h3>Where they came from</h3>
          <Bars rows={d.traffic.referrers.map((r) => ({ label: r.referrer, value: r.uniques }))} />
          <h3>What they looked at</h3>
          <Bars rows={d.traffic.paths.map((p) => ({ label: p.path.replace('/saqibsiddiq/QurbSpace', '') || '/', value: p.uniques }))} />
        </section>
      )}

      {d.notes.length > 0 && (
        <section className="db-notes">
          {d.notes.map((n) => (
            <p key={n}>{n}</p>
          ))}
        </section>
      )}
    </main>
  )
}
