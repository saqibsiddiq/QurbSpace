// What the dashboard shows, and where each figure comes from. Nothing here
// comes from anybody's devices: the apps report nothing (PRIVACY.md). It is
// what GitHub counts of the releases and the repository, and how many asked
// for the waitlist.

const REPO = 'saqibsiddiq/QurbSpace'
const API = 'https://api.github.com'

export type Platform = 'Android' | 'Windows' | 'Ubuntu and Debian' | 'Any Linux' | 'Phone updates' | 'Computer updates' | 'Other'

/** What a release file is, by its name (.github/workflows/release.yml). */
export function platformOf(name: string): Platform {
  if (name === 'android.json') return 'Phone updates'
  if (name === 'latest.json' || name.endsWith('.sig')) return 'Computer updates'
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
  const token = process.env.GITHUB_TOKEN
  const notes: string[] = []

  const releases: Release[] = []
  const totals: Partial<Record<Platform, number>> = {}
  try {
    const list = await github<GitHubRelease[]>(`/repos/${REPO}/releases?per_page=50`, token)
    for (const release of list) {
      const byPlatform: Partial<Record<Platform, number>> = {}
      for (const asset of release.assets) {
        const platform = platformOf(asset.name)
        byPlatform[platform] = (byPlatform[platform] ?? 0) + asset.download_count
        totals[platform] = (totals[platform] ?? 0) + asset.download_count
      }
      releases.push({ tag: release.tag_name, published: release.published_at, byPlatform })
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

  return { at: new Date().toISOString(), releases, totals, repo, traffic, waitlist, notes }
}
