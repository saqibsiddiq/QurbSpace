# The launch video

60 seconds, 16:9 (1920 × 1080), for the website, YouTube and the README.
No voice (the owner has no microphone): the story told in short words on
screen, set to the owner's music; real screens of the apps, recorded on the
owner's Galaxy S23 (the owner's face and some details are fine, nothing very
personal), in device frames, with animated titles in the brand (decision 0091).
Not "free": a beta should not say a price it may not keep.

## The script

On screen, a few words at a time, each held long enough to read. Every claim
is one the apps make good on today (README, *Known gaps*).

| # | Time | Words on screen | Picture |
|---|---|---|---|
| 1 | 0 to 5 s | *Your files. Always near.* | The two circles drift together until they almost touch; the wordmark appears. |
| 2 | 5 to 12 s | *Your phone and your computer. In sync. Nothing in between.* | Phone and laptop side by side; a line of light between them, no cloud above. |
| 3 | 12 to 20 s | *Send in seconds. Both ways.* | Real phone: Share, then the laptop. Real laptop: it arrives. Then the other way. |
| 4 | 20 to 28 s | *Back up WhatsApp and photos. To your own computer.* | Real phone: choosing folders; the backup bar filling. |
| 5 | 28 to 36 s | *Open your computer from your phone. From anywhere.* | Real phone: Files, the laptop, a folder, a photo opening. |
| 6 | 36 to 44 s | *Any network. Laptop asleep? Files wait in your Drive, sealed.* | The phone moving from Wi-Fi to mobile data, still connected; a sealed envelope into Drive. |
| 7 | 44 to 52 s | *Encrypted end to end. No account. No cloud of ours.* | Three short words in turn: Encrypted. No account. No cloud of ours. |
| 8 | 52 to 60 s | *QurbSpace. Open beta, for Android, Windows and Linux. qurb.cloud* | The logo, the three platforms, **qurb.cloud**. |

## Music

A royalty-free track the owner chooses (the YouTube Audio Library is free and
safe for YouTube and the website), calm and bright, about 100 to 120 BPM,
the cuts timed to its beat.

## How it is made

- **Screens**: the phone app recorded on the owner's S23 with
  `adb screenrecord`, sample photos (the distribution's wallpapers) for what
  is sent and opened, kept away from very personal files; the laptop's
  window from its design fixtures (`experiments/desktop-fixtures`), in a
  laptop frame.
- **Titles and device frames**: HTML scenes in the brand (Inter, the green,
  the mark), rendered frame by frame in Chromium and assembled with ffmpeg.
- **Everything** cut to the music, at 30 frames a second, encoded H.264 for
  the web, with a 1280 × 720 copy for the README.
