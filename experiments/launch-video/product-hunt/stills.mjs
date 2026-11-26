// Product Hunt gallery stills from the website's own scenes: each scrolled to
// the middle of the view, let play in real time until settled, captured at
// 1270x760 (Product Hunt's gallery size) at twice the pixels.
import { writeFileSync } from 'node:fs'
import { chrome } from '../cdp.mjs'
const OUT = process.argv[2]
const STOPS = [
  { name: 'pocket', sel: '.wt-scene', nth: 1, hold: 8 },
  { name: 'scan', sel: '.wt-scene', nth: 0, hold: 7 },
  { name: 'send', sel: '.sp', hold: 6 },
  { name: 'backups', sel: '.bk', hold: 6 },
  { name: 'vault', sel: '.vt', hold: 6 },
  { name: 'middle', sel: '.nm', hold: 6 },
]
const POLISH = `.qs-nav, .qs-foot, nextjs-portal, [data-nextjs-toast], #__next-build-watcher { visibility: hidden !important; } html { scroll-behavior: auto !important; }`
const c = await chrome({ width: 1270, height: 760, scale: 2 })
try {
  await c.open('http://localhost:3000/', 6000)
  await c.eval(`(() => { const s = document.createElement('style'); s.textContent = ${JSON.stringify(POLISH)}; document.head.append(s); return true })()`)
  for (const s of STOPS) {
    await c.eval(`(() => { const el = document.querySelectorAll('${s.sel}')[${s.nth || 0}]; const r = el.getBoundingClientRect();
      window.scrollTo(0, Math.max(0, r.top + scrollY + r.height / 2 - innerHeight / 2)); return true })()`)
    await new Promise((r) => setTimeout(r, s.hold * 1000))
    writeFileSync(`${OUT}/${s.name}.png`, await c.shot('png'))
    console.log('captured', s.name)
  }
} finally { c.close() }
