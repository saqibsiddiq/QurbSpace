// Render scenes.html to a video: every frame by `seek(t)`, captured, then
// encoded by ffmpeg with the music under it.
//
//   node render.mjs [out.mp4] [--music track.mp3] [--from 0 --to 60] [--fps 30]
import { mkdirSync, writeFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { join } from 'node:path'
import { chrome } from './cdp.mjs'

const args = process.argv.slice(2)
const opt = (name, fallback) => { const i = args.indexOf(`--${name}`); return i >= 0 ? args[i + 1] : fallback }
const out = args.find((a) => a.endsWith('.mp4')) || 'qurbspace-launch.mp4'
const fps = Number(opt('fps', 30))
const from = Number(opt('from', 0))
const music = opt('music', null)
const frames = join(process.env.FRAMES || '/tmp', `qurbspace-frames-${process.pid}`)
mkdirSync(frames, { recursive: true })

const c = await chrome({ width: 1920, height: 1080 })
try {
  await c.open('http://localhost:8731/experiments/launch-video/scenes.html', 1500)
  const to = Number(opt('to', await c.eval('window.DURATION')))
  const total = Math.round((to - from) * fps)
  const started = Date.now()
  for (let i = 0; i < total; i++) {
    await c.eval(`seek(${(from + i / fps).toFixed(4)})`)
    writeFileSync(join(frames, `${String(i).padStart(5, '0')}.jpg`), await c.shot('jpeg'))
    if (i % fps === 0) process.stdout.write(`\r${(i / fps).toFixed(0)} s of ${(total / fps).toFixed(0)}`)
  }
  console.log(`\rrendered ${total} frames in ${((Date.now() - started) / 1000).toFixed(0)} s`)
} finally { c.close() }

const audio = music ? ['-ss', String(from), '-i', music, '-filter_complex',
  `[1:a]afade=t=in:d=1,afade=t=out:st=${Math.max(0, Number(opt('to', 60)) - from - 2.5)}:d=2.5,loudnorm=I=-16:TP=-1.5[a]`, '-map', '0:v', '-map', '[a]', '-c:a', 'aac', '-b:a', '192k', '-shortest'] : []
execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', String(fps), '-i', join(frames, '%05d.jpg'), ...audio,
  '-c:v', 'libx264', '-preset', 'slow', '-crf', '17', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', out], { stdio: 'inherit' })
console.log('wrote', out)
