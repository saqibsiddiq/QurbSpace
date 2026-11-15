// A headless Chrome driven over the DevTools protocol, with Node's own
// WebSocket and fetch: nothing to install.
import { spawn } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

export async function chrome({ width, height, scale = 1 }) {
  const port = 9300 + Math.floor(Math.random() * 400)
  const proc = spawn('google-chrome-stable', [
    '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${mkdtempSync(join(tmpdir(), 'qv-'))}`,
    '--hide-scrollbars', '--mute-audio', '--autoplay-policy=no-user-gesture-required', '--force-color-profile=srgb',
    `--window-size=${width},${height}`, 'about:blank',
  ], { stdio: 'ignore' })
  let page
  for (let i = 0; i < 100 && !page; i++) {
    await new Promise((r) => setTimeout(r, 100))
    try { page = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page') } catch {}
  }
  if (!page) throw new Error('chrome did not start')
  const ws = new WebSocket(page.webSocketDebuggerUrl)
  await new Promise((ok) => ws.addEventListener('open', ok, { once: true }))
  let id = 0
  const waiting = new Map()
  ws.addEventListener('message', (m) => {
    const msg = JSON.parse(m.data)
    if (msg.id && waiting.has(msg.id)) { const { ok, no } = waiting.get(msg.id); waiting.delete(msg.id); msg.error ? no(new Error(msg.error.message)) : ok(msg.result) }
  })
  const send = (method, params = {}) => new Promise((ok, no) => { const n = ++id; waiting.set(n, { ok, no }); ws.send(JSON.stringify({ id: n, method, params })) })
  await send('Page.enable')
  await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: scale, mobile: false })
  return {
    send,
    async open(url, settleMs = 1500) {
      await send('Page.navigate', { url })
      await new Promise((r) => setTimeout(r, settleMs))
      await this.eval('document.fonts.ready.then(() => true)')
    },
    async eval(expression) {
      const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text)
      return r.result.value
    },
    async shot(format = 'png') {
      const r = await send('Page.captureScreenshot', { format, ...(format === 'jpeg' ? { quality: 95 } : {}) })
      return Buffer.from(r.data, 'base64')
    },
    close() { try { ws.close() } catch {} proc.kill() },
  }
}
