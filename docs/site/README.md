# BridgeLab landing page

Static single-page marketing site served from this folder via GitHub Pages
(see `.github/workflows/pages.yml`). Zero build step - plain HTML + CSS so
anyone can edit copy without a toolchain.

## Files

- `index.html` - page content (nav, hero, features, compare table, plugins,
  download, FAQ, footer)
- `style.css` - Catppuccin-inspired dark theme matching the app
- `favicon.svg` - 32x32 brand mark (bridge + stripe)
- `og-image.svg` - 1200x630 OG/Twitter preview card

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

The current OG image is an inline SVG. Social networks render SVG
inconsistently; when you have a real screenshot, export a `1200x630 PNG` at
`og-image.png` and update the `<meta property="og:image">` line in
`index.html`.
