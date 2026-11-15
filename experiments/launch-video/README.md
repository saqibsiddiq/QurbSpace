# The launch video

Throwaway tooling for the 60-second launch video (docs/brand/launch-video.md):
an HTML page that draws every scene from one clock, and a renderer that steps
that clock through headless Chrome and hands the frames to ffmpeg.

| File | What it does |
|---|---|
| `scenes.html` | The eight scenes. `seek(t)` sets every element for second `t`; phone clips and laptop screens come from `assets/`. |
| `cdp.mjs` | A headless Chrome over the DevTools protocol, with Node's own WebSocket: nothing to install. |
| `shots.mjs` | The laptop's screens, from `experiments/desktop-fixtures` (the real window on made-up data). |
| `render.mjs` | Frames at 30 a second, then ffmpeg: H.264, the music faded and levelled under it. |

Served by the repository's `python3 -m http.server 8731` (the `desktop-fixtures`
entry in `.claude/launch.json`), then:

```bash
node experiments/launch-video/shots.mjs
node experiments/launch-video/render.mjs qurbspace-launch.mp4 --music track.mp3
```

## Corners cut

- The phone clips are recorded by hand on the owner's S23 (`adb screenrecord`)
  and dropped into `assets/` as `phone-home.mp4`, `phone-send.mp4`,
  `phone-backup.mp4` and `phone-remote.mp4`; nothing records them for you.
- `assets/` and the rendered video are not committed: they are made from what
  is here and are large.
- Timings are fixed in `scenes.html`; changing the music means moving them by
  hand to its beat.
