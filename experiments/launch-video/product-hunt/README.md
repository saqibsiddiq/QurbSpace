# Product Hunt gallery

The five gallery images on the QurbSpace launch (2026-11-26), in their order
there: the website's own scenes, made-up data only, rendered by `stills.mjs` at
Product Hunt's 1270x760, twice the pixels. `gallery-preview.jpg` is the six
first rendered; the backups scene was left out (its title sits far left at
this size).

    npm --prefix website run dev      # port 3000
    node experiments/launch-video/product-hunt/stills.mjs experiments/launch-video/product-hunt

Corners cut: each scene is held a fixed number of seconds in real time, not on
the film clock `site.mjs` uses, so a slow machine may catch one unsettled.
