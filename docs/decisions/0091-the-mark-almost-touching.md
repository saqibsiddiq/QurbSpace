# 0091: The mark: two circles, almost touching

**Status:** Accepted, 2026-11-15. Replaces the mark of
[0048](0048-the-design-direction.md) (a ring with a stem making a q, and a
point of light inside); the rest of 0048 stands.

## What was asked

The owner, 2026-11-15: "i wanted to design a new logo". Asked what it should
say first, the owner chose **closeness** (Qurb means nearness), kept the green,
and wanted the app icon, a wordmark, and the website and GitHub images, drawn
as concepts in Figma first. Of six concepts (*Logo concepts: closeness* in the
Figma file's Foundations page): "go with almost touching". And: "the name of
the app should be qurbspace", written **QurbSpace**, where people see it.

## Decided

- **The mark**: two circles side by side, a hair apart, the first white and
  the second Qurb light (#BDE7D6), on the green tile (the gradient #3C806A to
  #22503F). In the mark's 32-unit box: the tile from 1 to 31 with corners of
  9, the circles of radius 4.92 at x = 10.73 and 21.27 on the middle line, so
  the gap is 0.7. Close, never merged.
- **The master** is `packaging/qurb.svg`; the window (`setup.js`), the website
  (`app/icon.svg`, `components/mark.tsx`), Android's launcher icon and glyph,
  the tray (`crates/tray/src/icon.rs`) and the desktop's `icon.png` and
  `icon.ico` are drawn from it.
- **In one colour at small sizes** (Android's status bar and notifications,
  `qurb_notification.xml`), the circles are larger and further apart, so they
  stay two at 24 dp.
- **The tray** shows state as before, by colour, and while working or in
  trouble the second circle is only its outline.
- **The wordmark**: QurbSpace in Inter Semi Bold, tracked in by 2.5 %, beside
  the mark (`docs/brand/`).

- **The name QurbSpace where people see it**: the apps' sentences and titles
  (the phone's Files root, the setup's directions, the storage page, the
  notifications' channel), the window's title, the Linux menu entry, the
  installer's publisher, the website, the Windows README. **Not** renamed, at
  the owner's choice: the `qurb` command and its output, the folders
  (`~/qurb`, `~/.config/qurb`, `Downloads/Qurb`), the download file names, and
  every internal name (the package `com.qurb`, settings and store names, style
  names, the Windows start-up entry), since renaming those would orphan what
  installed copies already hold.

## Consequences

- Installed apps show the new icon from the release that carries it.
- GitHub's social preview is set by hand in the repository's settings, from
  `docs/brand/github-social-preview.png`.
