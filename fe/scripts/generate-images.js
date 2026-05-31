#!/usr/bin/env node
// Generates brand assets from the source PNGs in repo-root /assets:
//
//   assets/sloth.png         → favicons + responsive sloth icon
//   assets/sloth-family.png  → Open Graph image + Home-hero webp
//   assets/example-*.png     → marketing tree-screenshot webps
//
// Run via `pnpm generate:images` (wired before `vite build` in the fe
// package.json). Idempotent — safe to re-run; sharp overwrites in place.
//
// Output landing zones:
//   - fe/public/brand/  — files referenced from index.html (favicons +
//     apple-touch-icon + favicon.ico). Vite copies these verbatim, with
//     stable URLs the index.html `<link>` tags point at. nginx serves
//     them with a 1-day must-revalidate cache (see .docker/fe/nginx.conf).
//   - fe/src/assets/brand/ — files imported from Vue components (sloth-
//     128/256/512.webp, sloth-family-960.webp, tree-example-*.webp,
//     og-1200x630.png). Vite hashes the filename to `[name]-[hash].ext`
//     and rewrites every import site, so a regenerated image lands at a
//     new URL → browsers + CDNs fetch fresh automatically without any
//     cache-control bypass. nginx serves /assets/* with the year-long
//     immutable cache.
//
// The split exists so a brand-asset refresh (`pnpm run generate:images`
// → commit → deploy) propagates to every user immediately for the
// component-referenced imagery — the symptom the user hit with the
// sloth-family.png update where prod kept serving the old cached webp.

import sharp from 'sharp'
import { promises as fs } from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// Repo layout: this script lives at fe/scripts/. The fe directory is
// the script's parent; the repo-root `assets/` is `fe/../assets`.
//
// We anchor on `feRoot` rather than a notional repoRoot/fe because the
// docker-compose fe service mounts the repo-root `fe/` directory as
// `/app` — there is no `/fe` inside the container, only `/app` and a
// peer `/assets` mount (compose.yaml). The same paths resolve correctly
// in CI where the script runs from a real repo checkout.
const feRoot = path.resolve(__dirname, '..')
const assetsDir = path.resolve(feRoot, '..', 'assets')
const publicBrandDir = path.join(feRoot, 'public', 'brand')
const srcBrandDir = path.join(feRoot, 'src', 'assets', 'brand')

// `sloth-center.png` is the centred / tighter crop of the original
// sloth — favicons + the in-app icon all derive from it so the head
// stays in frame at every size. The family group lives in
// `sloth-family.png` (used for the Home hero + the OG card).
// `example-light.png` / `example-dark.png` are real screenshots of the
// tree view in the respective theme — the marketing page swaps between
// them based on the visitor's resolved theme so the screenshot matches
// the surrounding chrome.
const SLOTH = path.join(assetsDir, 'sloth-center.png')
const SLOTH_FAMILY = path.join(assetsDir, 'sloth-family.png')
const TREE_EXAMPLE_LIGHT = path.join(assetsDir, 'example-light.png')
const TREE_EXAMPLE_DARK = path.join(assetsDir, 'example-dark.png')

async function ensureDir(dir) {
    await fs.mkdir(dir, { recursive: true })
}

// The sloth source is 1536 × 1024 — not square. Favicons need square,
// so we centre-crop the source to 1024 × 1024 first, then resize per
// target. Doing this once keeps every favicon size visually consistent.
async function loadSlothSquare() {
    const meta = await sharp(SLOTH).metadata()
    const side = Math.min(meta.width, meta.height)
    const left = Math.floor((meta.width - side) / 2)
    const top = Math.floor((meta.height - side) / 2)
    return sharp(SLOTH).extract({ left, top, width: side, height: side })
}

async function generateFavicons() {
    // PNG favicons. Browsers prefer the SVG declared in index.html;
    // these are the legacy/static-tab fallbacks. STAYS in public/ —
    // index.html `<link rel="icon" href="/brand/favicon-32.png" />`
    // expects the URL to be stable.
    for (const size of [16, 32, 48]) {
        const out = path.join(publicBrandDir, `favicon-${size}.png`)
        await (await loadSlothSquare()).resize(size, size).png().toFile(out)
        console.log(`  public/brand/favicon-${size}.png`)
    }

    // .ico: most browsers (Chromium, Firefox, Safari) accept a PNG body
    // with the .ico extension. Sticking with PNG-as-ICO avoids the
    // sharp-ico devDependency churn for a single artefact.
    const ico = path.join(publicBrandDir, 'favicon.ico')
    await (await loadSlothSquare()).resize(32, 32).png().toFile(ico)
    console.log('  public/brand/favicon.ico (32 × 32 PNG)')

    // Apple touch icon — iOS home-screen tile.
    const apple = path.join(publicBrandDir, 'apple-touch-icon.png')
    await (await loadSlothSquare()).resize(180, 180).png().toFile(apple)
    console.log('  public/brand/apple-touch-icon.png (180 × 180)')
}

async function generateSlothResponsive() {
    // Responsive sloth icon (sidebar, AppBar, anywhere the lockup needs
    // a raster fallback). All from the centre-cropped square so the
    // shape is consistent at every size. Lands in src/assets/ — Vite
    // hashes the filename so a regenerated sloth busts the browser /
    // CDN cache automatically.
    for (const size of [128, 256, 512]) {
        const out = path.join(srcBrandDir, `sloth-${size}.webp`)
        await (await loadSlothSquare()).resize(size, size).webp({ quality: 90 }).toFile(out)
        console.log(`  src/assets/brand/sloth-${size}.webp`)
    }
}

async function generateOgImage() {
    // Open Graph spec: 1200 × 630 (ratio 1.91). Source sloth-family is
    // 1536 × 1024 (ratio 1.5). `fit: 'cover'` centre-crops top/bottom
    // off the source — the sloth family stays middle of frame.
    //
    // Hashed via src/assets/ so a refreshed OG image lands at a new URL,
    // forcing social-media crawlers to re-fetch on the next crawl
    // instead of serving the prior URL's cached card forever.
    const out = path.join(srcBrandDir, 'og-1200x630.png')
    await sharp(SLOTH_FAMILY).resize(1200, 630, { fit: 'cover', position: 'centre' }).png({ quality: 92 }).toFile(out)
    console.log('  src/assets/brand/og-1200x630.png')
}

async function generateHeroImage() {
    // Home hero image. 960 wide is plenty for the half-column slot on
    // desktop; the SrcSet declared in the Home view can ask for 640 +
    // 1280 variants later if needed without touching this script.
    const out = path.join(srcBrandDir, 'sloth-family-960.webp')
    await sharp(SLOTH_FAMILY).resize(960, null, { withoutEnlargement: true }).webp({ quality: 88 }).toFile(out)
    console.log('  src/assets/brand/sloth-family-960.webp')
}

async function generateTreeExample() {
    // Two widths per theme so retina screens get a crisper render AND
    // the HomeView's `<picture>` can pick the variant that matches the
    // visitor's resolved theme. Output filenames carry the theme
    // suffix: `tree-example-light-960.webp`, `tree-example-dark-1280.webp`.
    const variants = [
        { src: TREE_EXAMPLE_LIGHT, theme: 'light' },
        { src: TREE_EXAMPLE_DARK, theme: 'dark' },
    ]
    for (const { src, theme } of variants) {
        for (const width of [960, 1280]) {
            const out = path.join(srcBrandDir, `tree-example-${theme}-${width}.webp`)
            await sharp(src).resize(width, null, { withoutEnlargement: true }).webp({ quality: 90 }).toFile(out)
            console.log(`  src/assets/brand/tree-example-${theme}-${width}.webp`)
        }
    }
}

async function main() {
    console.log('Generating brand assets …')
    await ensureDir(publicBrandDir)
    await ensureDir(srcBrandDir)
    await generateFavicons()
    await generateSlothResponsive()
    await generateOgImage()
    await generateHeroImage()
    await generateTreeExample()
    console.log('Done.')
}

main().catch((err) => {
    console.error('Image generation failed:', err)
    process.exit(1)
})
