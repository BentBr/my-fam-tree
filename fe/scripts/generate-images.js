#!/usr/bin/env node
// Generates brand assets from the two source PNGs in repo-root /assets:
//
//   assets/sloth.png         → favicons + responsive sloth icon
//   assets/sloth-family.png  → Open Graph image + Home-hero webp
//
// Run via `pnpm generate:images` (wired before `vite build` in the fe
// package.json). Idempotent — safe to re-run; sharp overwrites in place.
//
// Output landing zone: fe/public/brand/. Vite picks the directory up as
// static at build time (everything in fe/public/ ships verbatim).

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
const outDir = path.join(feRoot, 'public', 'brand')

// `sloth-center.png` is the centred / tighter crop of the original
// sloth — favicons + the in-app icon all derive from it so the head
// stays in frame at every size. The family group lives in
// `sloth-family.png` (used for the Home hero + the OG card).
// `example.png` is a real screenshot of the tree view, used to anchor
// the marketing page's "this is what you get" slot.
const SLOTH = path.join(assetsDir, 'sloth-center.png')
const SLOTH_FAMILY = path.join(assetsDir, 'sloth-family.png')
const TREE_EXAMPLE = path.join(assetsDir, 'example.png')

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
    // these are the legacy/static-tab fallbacks.
    for (const size of [16, 32, 48]) {
        const out = path.join(outDir, `favicon-${size}.png`)
        await (await loadSlothSquare()).resize(size, size).png().toFile(out)
        console.log(`  favicon-${size}.png`)
    }

    // .ico: most browsers (Chromium, Firefox, Safari) accept a PNG body
    // with the .ico extension. Sticking with PNG-as-ICO avoids the
    // sharp-ico devDependency churn for a single artefact.
    const ico = path.join(outDir, 'favicon.ico')
    await (await loadSlothSquare()).resize(32, 32).png().toFile(ico)
    console.log('  favicon.ico (32 × 32 PNG)')

    // Apple touch icon — iOS home-screen tile.
    const apple = path.join(outDir, 'apple-touch-icon.png')
    await (await loadSlothSquare()).resize(180, 180).png().toFile(apple)
    console.log('  apple-touch-icon.png (180 × 180)')
}

async function generateSlothResponsive() {
    // Responsive sloth icon (sidebar, AppBar, anywhere the lockup needs
    // a raster fallback). All from the centre-cropped square so the
    // shape is consistent at every size.
    for (const size of [128, 256, 512]) {
        const out = path.join(outDir, `sloth-${size}.webp`)
        await (await loadSlothSquare()).resize(size, size).webp({ quality: 90 }).toFile(out)
        console.log(`  sloth-${size}.webp`)
    }
}

async function generateOgImage() {
    // Open Graph spec: 1200 × 630 (ratio 1.91). Source sloth-family is
    // 1536 × 1024 (ratio 1.5). `fit: 'cover'` centre-crops top/bottom
    // off the source — the sloth family stays middle of frame.
    const out = path.join(outDir, 'og-1200x630.png')
    await sharp(SLOTH_FAMILY).resize(1200, 630, { fit: 'cover', position: 'centre' }).png({ quality: 92 }).toFile(out)
    console.log('  og-1200x630.png')
}

async function generateHeroImage() {
    // Home hero image. 960 wide is plenty for the half-column slot on
    // desktop; the SrcSet declared in the Home view can ask for 640 +
    // 1280 variants later if needed without touching this script.
    const out = path.join(outDir, 'sloth-family-960.webp')
    await sharp(SLOTH_FAMILY).resize(960, null, { withoutEnlargement: true }).webp({ quality: 88 }).toFile(out)
    console.log('  sloth-family-960.webp')
}

async function generateTreeExample() {
    // Two widths so retina screens get a crisper render. Source is
    // 1245 × 732, so both targets stay within the source resolution.
    for (const width of [960, 1280]) {
        const out = path.join(outDir, `tree-example-${width}.webp`)
        await sharp(TREE_EXAMPLE).resize(width, null, { withoutEnlargement: true }).webp({ quality: 90 }).toFile(out)
        console.log(`  tree-example-${width}.webp`)
    }
}

async function main() {
    console.log('Generating brand assets into fe/public/brand/ …')
    await ensureDir(outDir)
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
