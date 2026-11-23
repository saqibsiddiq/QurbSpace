// The newest release as GitHub has it, for the download page's version and
// file sizes. Asked at most once an hour; without an answer the page shows
// neither, and every button still goes to the newest file (site-config).

export type Release = {
  version: string
  published: string
  /** Bytes, by the names without a version (.github/workflows/release.yml). */
  sizes: Partial<Record<'android' | 'windows' | 'deb' | 'appImage', number>>
}

const FILES = {
  'QurbSpace-android.apk': 'android',
  'QurbSpace-windows-setup.exe': 'windows',
  'QurbSpace-linux.deb': 'deb',
  'QurbSpace-linux.AppImage': 'appImage'
} as const

export async function latestRelease(): Promise<Release | null> {
  try {
    const token = process.env.GITHUB_TOKEN?.trim()
    const response = await fetch('https://api.github.com/repos/saqibsiddiq/QurbSpace/releases/latest', {
      headers: {
        Accept: 'application/vnd.github+json',
        ...(token ? { Authorization: `Bearer ${token}` } : {})
      },
      next: { revalidate: 3600 }
    })
    if (!response.ok) return null
    const r = (await response.json()) as { tag_name: string; published_at: string; assets: { name: string; size: number }[] }
    const sizes: Release['sizes'] = {}
    for (const asset of r.assets) {
      const key = FILES[asset.name as keyof typeof FILES]
      if (key) sizes[key] = asset.size
    }
    return { version: r.tag_name.replace(/^v/, ''), published: r.published_at, sizes }
  } catch {
    return null
  }
}

export const megabytes = (bytes?: number) => (bytes ? `${Math.round(bytes / 1_000_000)} MB` : null)
