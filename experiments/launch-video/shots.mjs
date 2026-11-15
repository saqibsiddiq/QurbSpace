// The laptop's screens for the video, from the desktop window's design
// fixtures (experiments/desktop-fixtures): the real interface on made-up
// data. Served by `python3 -m http.server 8731` at the repository's root.
import { writeFileSync } from 'node:fs'
import { chrome } from './cdp.mjs'

const BASE = 'http://localhost:8731/experiments/desktop-fixtures/index.html'
const SHOTS = {
  'laptop-home.png': `${BASE}?still`,
  'laptop-received.png': `${BASE}?still&then=${encodeURIComponent('go("received")')}`,
  'laptop-backups.png': `${BASE}?still&dev=4cef0d89`,
  'laptop-files.png': `${BASE}?still&then=${encodeURIComponent('go("files")')}`,
}
const c = await chrome({ width: 1040, height: 650, scale: 2 })
try {
  for (const [name, url] of Object.entries(SHOTS)) {
    await c.open(url, 2600)
    writeFileSync(new URL(`assets/${name}`, import.meta.url), await c.shot())
    console.log('wrote', name)
  }
} finally { c.close() }
