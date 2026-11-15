# Brand

The mark (decision 0091): two circles almost touching, one white and one Qurb
light, on the green tile. Qurb means nearness: close, never merged.

| File | What it is |
|---|---|
| `mark.svg` | The mark, a copy of `packaging/qurb.svg`, the master every icon is drawn from |
| `readme-header-light.png`, `readme-header-dark.png` | The mark and the wordmark, for the README; their `.svg` sources set the name in Inter Semi Bold |
| `github-social-preview.png` | 1280 × 640, for the repository's social preview (Settings → General → Social preview) |

The concepts and the final sheet are in the Figma file's Foundations page:
*Logo concepts: closeness* and *Logo: Almost touching*.

To render the headers again, with the website's Inter (`website/public/fonts/inter-latin.woff2`)
decompressed to a `.ttf` and made visible to fontconfig:

```bash
rsvg-convert docs/brand/readme-header-light.svg -o docs/brand/readme-header-light.png
```
