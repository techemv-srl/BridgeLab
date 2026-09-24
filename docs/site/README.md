# BridgeLab landing page

Static single-page marketing site served from this folder via GitHub Pages
(see `.github/workflows/pages.yml`). Zero build step - plain HTML + CSS so
anyone can edit copy without a toolchain.

## Files

- `index.html` - page content (nav, hero, features, compare table, plugins,
  download, FAQ, footer)
- `style.css` - Catppuccin-inspired dark theme matching the app
- `favicon.svg` - 32x32 brand mark (bridge + stripe)
- `og-image.svg` - 1200x627 (1.91:1) OG/Twitter preview card, the source
- `og-image.png` - the same card rendered to PNG; this is what the meta tags point at

## Local preview

```bash
# Any static server works; pick one:
python3 -m http.server --directory docs/site 4173
# or
npx serve docs/site
```

Then open http://localhost:4173

## Publish

Pushing to `main` with changes under `docs/site/**` triggers the workflow,
which uploads the folder to GitHub Pages. The first time, enable Pages in
**repo Settings > Pages** and set source to "GitHub Actions".

Custom domain: add a `CNAME` file in `docs/site/` with the FQDN (e.g.
`bridgelab.dev`) and configure the DNS `CNAME` record to
`1warpengine.github.io`.

## Editing copy

All text lives in `index.html` - search for the section heading and edit in
place. Feature list is inside `<section id="features">`, the comparison
table inside `<section id="compare">`, etc.

Download links point at `https://github.com/1warpengine/HL7_editor/releases/latest`
- they resolve to the current release automatically once tags exist.

## OG image

Social networks do not render SVG previews reliably, so the meta tags in
`index.html` point at `og-image.png` (1200x627, 1.91:1) by **absolute URL**
— crawlers do not resolve relative paths. `og-image.svg` is the source:
keep everything that matters inside the central 80% (x 120-1080,
y 63-564), because preview cards crop the edges. After editing it,
re-render the PNG with the text in Inter (a headless Chromium screenshot
at 1200x627 with the Inter woff2 files embedded via `@font-face` works;
system fallbacks change the letter widths). The tags must stay in the
static HTML: crawlers do not run JavaScript.
