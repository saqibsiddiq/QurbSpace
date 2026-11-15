import type { Metadata, Viewport } from 'next'

/** Where the site is served. Vercel redirects qurb.cloud here, so this is the
 *  canonical address, not the bare domain. */
export const siteUrl = 'https://www.qurb.cloud'

/** Where the code is. Public, so every page may link into it. */
export const repoUrl = 'https://github.com/saqibsiddiq/QurbSpace'

/** The newest release's files, by the names each release also carries without
 *  its version (.github/workflows/release.yml), so a link never goes stale. */
export const latestFile = (name: string) => `${repoUrl}/releases/latest/download/${name}`
export const downloads = {
  android: latestFile('QurbSpace-android.apk'),
  windows: latestFile('QurbSpace-windows-setup.exe'),
  deb: latestFile('QurbSpace-linux.deb'),
  appImage: latestFile('QurbSpace-linux.AppImage')
}

/** A document in the repository, as GitHub shows it. */
export const repoDoc = (path: string) => `${repoUrl}/blob/main/${path}`

const description =
  'QurbSpace keeps your files on your own devices and moves them straight between them, encrypted. No cloud in the middle keeps a copy.'

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: {
    default: 'QurbSpace: Your files. Always near.',
    template: '%s · QurbSpace'
  },
  description,
  applicationName: 'QurbSpace',
  keywords: ['private file sync', 'peer-to-peer sync', 'end-to-end encrypted', 'Linux', 'Android', 'Dropbox alternative'],
  openGraph: {
    title: 'QurbSpace',
    description: 'Your files. Always near.',
    url: siteUrl,
    siteName: 'QurbSpace',
    type: 'website'
  },
  twitter: {
    card: 'summary_large_image',
    title: 'QurbSpace',
    description: 'Your files. Always near.'
  },
  alternates: {
    canonical: siteUrl
  },
  robots: {
    index: true,
    follow: true
  }
}

export const viewport: Viewport = {
  themeColor: '#f7f7f4',
  width: 'device-width',
  initialScale: 1
}
