// What the dashboard shows, and where each figure comes from. Nothing here
// comes from anybody's devices: the apps report nothing (PRIVACY.md). It is
// what GitHub counts of the releases and the repository, and how many asked
// for the waitlist.

const REPO = 'saqibsiddiq/QurbSpace'
const API = 'https://api.github.com'

export type Platform =
  | 'Android'
  | 'Windows'
  | 'Ubuntu and Debian'
  | 'Any Linux'
  | 'Phone updates'
  | 'Computer updates'
  | 'Phone update checks'
  | 'Computer update checks'
  | 'Other'

/** Whether a file's name carries its version (qurb_0.2.0-beta.13_x64-setup.exe,
 *  QurbSpace-0.2.0-beta.13-android.apk): the files the apps update from
 *  (latest.json, android.json). People are sent to the names without one. */
const versioned = (name: string) => name.startsWith('qurb_') || /^QurbSpace-\d/.test(name)

/** What a release file is, by its name (.github/workflows/release.yml). In a
 *  release that also carries the names without a version, a versioned file is
 *  an app updating itself, not a person; before those names (beta.9 and
 *  earlier) people were sent to the versioned ones. */
export function platformOf(name: string, plainNames: boolean): Platform {
  if (name === 'android.json') return 'Phone update checks'
  if (name === 'latest.json') return 'Computer update checks'
  if (name.endsWith('.sig')) return 'Other'
  if (plainNames && versioned(name)) return name.endsWith('.apk') ? 'Phone updates' : 'Computer updates'
  if (name.endsWith('.apk')) return 'Android'
  if (name.endsWith('.exe') || name.endsWith('.msi')) return 'Windows'
  if (name.endsWith('.deb')) return 'Ubuntu and Debian'
  if (name.endsWith('.AppImage')) return 'Any Linux'
  return 'Other'
}

export type Release = {
  tag: string
  published: string
  byPlatform: Partial<Record<Platform, number>>
  /** Downloads by QurbSpace's own platform tests, left out of byPlatform. */
  tests: number
}

export type Traffic = {
  views: { count: number; uniques: number }
  clones: { count: number; uniques: number }
  days: { day: string; views: number; uniques: number }[]
  referrers: { referrer: string; count: number; uniques: number }[]
  paths: { path: string; count: number; uniques: number }[]
}

export type Dashboard = {
  at: string
  releases: Release[]
  totals: Partial<Record<Platform, number>>
  /** Downloads by QurbSpace's own platform tests, left out of the totals. */
  tests: number
  repo: { stars: number; forks: number; watchers: number; issues: number } | null
  traffic: Traffic | null
  waitlist: number | null
  notes: string[]
}

async function github<T>(path: string, token?: string): Promise<T> {
  const response = await fetch(`${API}${path}`, {
    headers: {
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      ...(token ? { Authorization: `Bearer ${token}` } : {})
    },
    // Fresh every five minutes at most: GitHub limits how often it is asked.
    next: { revalidate: 300 }
  })
  if (!response.ok) throw new Error(`GitHub ${path}: ${response.status}`)
  return response.json() as Promise<T>
}

type GitHubRelease = { tag_name: string; published_at: string; assets: { name: string; download_count: number }[] }
type GitHubRun = { id: number; created_at: string }
type GitHubJob = { name: string; steps?: { name: string; conclusion: string | null }[] }

/** What the platform tests downloaded (.github/workflows/platforms.yml), by
 *  the release each run tested: the newest published before it began, as a
 *  run tests the latest release or the one just made. Each run fetches the
 *  AppImage and the .deb on six Linux systems, the .deb once more for the
 *  window, and the Windows installer: about fourteen downloads a run, which
 *  until 2026-11-22 were shown as people. Counted from the steps that ran. */
async function testDownloads(
  releases: { tag: string; published: string }[],
  token?: string
): Promise<Map<string, Partial<Record<Platform, number>>>> {
  const byTag = new Map<string, Partial<Record<Platform, number>>>()
  const newestFirst = [...releases].sort((a, b) => b.published.localeCompare(a.published))
  const { workflow_runs: runs } = await github<{ workflow_runs: GitHubRun[] }>(
    `/repos/${REPO}/actions/workflows/platforms.yml/runs?per_page=100`,
    token
  )
  await Promise.all(
    runs.map(async (run) => {
      const release = newestFirst.find((r) => r.published <= run.created_at)
      if (!release) return
      const { jobs } = await github<{ jobs: GitHubJob[] }>(
        `/repos/${REPO}/actions/runs/${run.id}/jobs?filter=all&per_page=100`,
        token
      )
      const counts = byTag.get(release.tag) ?? {}
      const add = (p: Platform) => (counts[p] = (counts[p] ?? 0) + 1)
      for (const job of jobs) {
        const step = (name: string) => job.steps?.find((s) => s.name === name)?.conclusion
        const ran = (c: string | null | undefined) => c === 'success' || c === 'failure'
        if (job.name.startsWith('linux-start')) {
          const c = step('The downloads')
          if (ran(c)) add('Any Linux')
          if (c === 'success') add('Ubuntu and Debian')
        } else if (job.name === 'linux-window') {
          if (ran(step('Libraries, the .deb, and what drives the window'))) add('Ubuntu and Debian')
        } else if (job.name === 'windows-install') {
          if (ran(step('The installer'))) add('Windows')
        }
      }
      byTag.set(release.tag, counts)
    })
  )
  return byTag
}

type GitHubRepo = { stargazers_count: number; forks_count: number; subscribers_count: number; open_issues_count: number }

async function waitlistSize(): Promise<number | null> {
  const url = process.env.KV_REST_API_URL ?? process.env.UPSTASH_REDIS_REST_URL
  const token = process.env.KV_REST_API_TOKEN ?? process.env.UPSTASH_REDIS_REST_TOKEN
  if (!url || !token) return null
  const response = await fetch(url, {
    method: 'POST',
    headers: { authorization: `Bearer ${token}` },
    body: JSON.stringify(['SCARD', 'waitlist']),
    cache: 'no-store'
  })
  if (!response.ok) return null
  const { result } = (await response.json()) as { result: number }
  return result
}

export async function load(): Promise<Dashboard> {
  // Trimmed: a token pasted at Vercel's prompt can bring a line break.
  const token = process.env.GITHUB_TOKEN?.trim() || undefined
  const notes: string[] = []

  const releases: Release[] = []
  const totals: Partial<Record<Platform, number>> = {}
  let tests = 0
  try {
    const list = await github<GitHubRelease[]>(`/repos/${REPO}/releases?per_page=50`, token)
    let tested = new Map<string, Partial<Record<Platform, number>>>()
    try {
      tested = await testDownloads(list.map((r) => ({ tag: r.tag_name, published: r.published_at })), token)
    } catch (e) {
      notes.push(`The platform tests could not be read, so their downloads are counted as people: ${(e as Error).message}`)
    }
    for (const release of list) {
      const plainNames = release.assets.some((a) => a.name.startsWith('QurbSpace-') && !versioned(a.name))
      const byPlatform: Partial<Record<Platform, number>> = {}
      for (const asset of release.assets) {
        const platform = platformOf(asset.name, plainNames)
        byPlatform[platform] = (byPlatform[platform] ?? 0) + asset.download_count
      }
      let left = 0
      for (const [platform, n] of Object.entries(tested.get(release.tag_name) ?? {}) as [Platform, number][]) {
        const taken = Math.min(n, byPlatform[platform] ?? 0)
        byPlatform[platform] = (byPlatform[platform] ?? 0) - taken
        left += taken
      }
      for (const [platform, n] of Object.entries(byPlatform) as [Platform, number][]) {
        totals[platform] = (totals[platform] ?? 0) + n
      }
      tests += left
      releases.push({ tag: release.tag_name, published: release.published_at, byPlatform, tests: left })
    }
  } catch (e) {
    notes.push(`Releases could not be read: ${(e as Error).message}`)
  }

  let repo: Dashboard['repo'] = null
  try {
    const r = await github<GitHubRepo>(`/repos/${REPO}`, token)
    repo = { stars: r.stargazers_count, forks: r.forks_count, watchers: r.subscribers_count, issues: r.open_issues_count }
  } catch (e) {
    notes.push(`The repository could not be read: ${(e as Error).message}`)
  }

  // Only the repository's owner may read its traffic: a token is needed.
  let traffic: Traffic | null = null
  if (token) {
    try {
      const [views, clones, referrers, paths] = await Promise.all([
        github<{ count: number; uniques: number; views: { timestamp: string; count: number; uniques: number }[] }>(
          `/repos/${REPO}/traffic/views`,
          token
        ),
        github<{ count: number; uniques: number }>(`/repos/${REPO}/traffic/clones`, token),
        github<{ referrer: string; count: number; uniques: number }[]>(`/repos/${REPO}/traffic/popular/referrers`, token),
        github<{ path: string; count: number; uniques: number }[]>(`/repos/${REPO}/traffic/popular/paths`, token)
      ])
      traffic = {
        views: { count: views.count, uniques: views.uniques },
        clones: { count: clones.count, uniques: clones.uniques },
        days: views.views.map((v) => ({ day: v.timestamp.slice(0, 10), views: v.count, uniques: v.uniques })),
        referrers,
        paths
      }
    } catch (e) {
      notes.push(`Traffic could not be read: ${(e as Error).message}`)
    }
  } else {
    notes.push('Visits to GitHub need GITHUB_TOKEN set in Vercel (a token that may read this repository).')
  }

  let waitlist: number | null = null
  try {
    waitlist = await waitlistSize()
    if (waitlist === null) notes.push('The waitlist is not connected here.')
  } catch (e) {
    notes.push(`The waitlist could not be read: ${(e as Error).message}`)
  }

  return { at: new Date().toISOString(), releases, totals, tests, repo, traffic, waitlist, notes }
}
