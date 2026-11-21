import { computed, toValue } from 'vue'
import type { MaybeRefOrGetter } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useHead } from '@unhead/vue'

import { useLocaleStore } from '@/stores/locale'
import type { SupportedLocale } from '@/stores/locale'

/**
 * Per-page head metadata — the single source of truth for `<title>`,
 * `<meta name="description">`, the OpenGraph / Twitter card block,
 * `<link rel="canonical">`, and the hreflang alternates.
 *
 * Every view (public *and* authenticated) gets its metadata through
 * this one composable rather than hand-rolling a `useHead` call —
 * "never copy code, abstract it". Callers hand us i18n KEYS via a
 * descriptor (which may itself be reactive); we resolve them through
 * `t()` inside reactive getters so the document head re-renders whenever
 * the locale switches OR the active route's descriptor changes. We
 * register exactly ONE `useHead`, driven by computed arrays, so this is
 * safe to mount once at the app root.
 *
 * Two flavours, chosen by `noindex`:
 *
 *   - **Indexable page** (the public home, every authenticated view):
 *     full title + description + the complete OG/Twitter set + canonical
 *     (+ hreflang when requested). Authenticated views are not crawled,
 *     but the spec asks for "SEO metas for ALL pages", so they still get
 *     a translated title + description + og block (cheap, consistent, and
 *     means a shared link to e.g. `/account` still previews sensibly).
 *
 *   - **noindex page** (`/imprint`, `/data-policy`): per the plan, ONLY
 *     a minimal title + `<meta name="robots" content="noindex,nofollow">`.
 *     No description, no OG, no canonical, no hreflang — these are legal
 *     pages we deliberately keep out of every index and preview.
 */
export interface PageMetaDescriptor {
    /** i18n key for the page-specific title segment (required). */
    titleKey: string
    /** i18n key for the meta description. Omitted on noindex pages. */
    descriptionKey?: string
    /**
     * Legal / non-indexable page. When `true` we emit only the title +
     * `robots: noindex,nofollow` and skip description/OG/canonical/hreflang.
     */
    noindex?: boolean
    /**
     * Absolute path used for `og:url` + `<link rel="canonical">` +
     * hreflang. Defaults to `/` when omitted; the route-meta dispatcher
     * fills in the live route path for pages that don't pin one.
     */
    canonicalPath?: string
    /**
     * Emit hreflang alternates (`en` / `de` / `x-default`). Only the
     * truly public, indexable pages want these; defaults to `false`.
     */
    hreflang?: boolean
}

// Brand wordmark appended to every tab title so the browser tab always
// carries the product name regardless of which page key is active.
const BRAND_SUFFIX = 'My Family Tree'

// Stable, NON-hashed OG image served from `fe/public/brand/`. It must be
// an absolute URL (card validators reject relative ones) and must not be
// content-hashed (the baked `index.html` fallback references the same
// fixed path, so a scraper that never runs our JS still resolves it).
const OG_IMAGE_PATH = '/brand/og-1200x630.png'

// Public origin the site is served from. Baked at build time from
// `VITE_BASE_URL` (CI sets it); falls back to the production domain so a
// local `pnpm build` still emits sensible absolute URLs.
const PUBLIC_BASE_URL: string =
    (import.meta.env['VITE_BASE_URL'] as string | undefined)?.replace(/\/$/, '') ?? 'https://my-fam-tree.eu'

const OG_IMAGE_URL = `${PUBLIC_BASE_URL}${OG_IMAGE_PATH}`

function ogLocaleFor(locale: SupportedLocale): string {
    return locale === 'de' ? 'de_DE' : 'en_US'
}

/** Absolute URL for an app path (`canonicalPath` override case). */
function canonicalUrlFor(path: string | undefined): string {
    const p = path ?? '/'
    return `${PUBLIC_BASE_URL}${p === '/' || p === '' ? '/' : p}`
}

/**
 * Strip a leading `/en` or `/de` locale segment off a public route path,
 * returning the locale-agnostic remainder (`''` for the home page). The
 * public site is locale-PREFIXED, so `/de/imprint` → `/imprint`,
 * `/en` → ``. Used to build the per-locale canonical + hreflang URLs.
 */
function stripLocalePrefix(path: string): string {
    const stripped = path.replace(/^\/(en|de)(?=\/|$)/, '')
    return stripped === '/' ? '' : stripped
}

/** Locale-prefixed absolute URL: `https://base/de/imprint`, `https://base/en`. */
function localizedUrl(locale: SupportedLocale, suffix: string): string {
    return `${PUBLIC_BASE_URL}/${locale}${suffix}`
}

// Same story for `ResolvableMeta`: a union whose arms key on `name` XOR
// `property` (a both-optional shape matches no arm and falls into the
// charset arm's requirements). Each tag declares exactly one.
type MetaTag = { name: string; content: string } | { property: string; content: string }
// unhead's `ResolvableLink` is a discriminated union: the `rel: "alternate"`
// arm REQUIRES a `type`. Literal rel + explicit type keep us assignable.
type LinkTag =
    { rel: 'canonical'; href: string } | { rel: 'alternate'; href: string; hreflang: string; type: 'text/html' }

/**
 * Wire the document head for the active page. Accepts a (possibly
 * reactive) descriptor; intended to be called ONCE — pass a getter from
 * the route-meta dispatcher so navigations re-target the same head.
 */
export function usePageMeta(descriptor: MaybeRefOrGetter<PageMetaDescriptor>): void {
    const { t } = useI18n()
    const locale = useLocaleStore()
    const route = useRoute()

    const d = computed<PageMetaDescriptor>(() => toValue(descriptor))

    const pageTitle = computed(() => `${t(d.value.titleKey)} · ${BRAND_SUFFIX}`)

    // Canonical URL. For the locale-prefixed public pages we build it from
    // the live route path so it carries the right `/en` or `/de` prefix
    // and survives the static prerender (where the path IS the locale
    // source of truth). A descriptor may pin an explicit `canonicalPath`
    // for non-public pages that want a fixed self-canonical.
    const localeSuffix = computed(() => stripLocalePrefix(route.path))
    const canonical = computed(() =>
        d.value.canonicalPath !== undefined
            ? canonicalUrlFor(d.value.canonicalPath)
            : localizedUrl(locale.locale, localeSuffix.value),
    )

    const metaTags = computed<MetaTag[]>(() => {
        if (d.value.noindex === true) {
            return [{ name: 'robots', content: 'noindex, nofollow' }]
        }
        const description =
            d.value.descriptionKey !== undefined ? t(d.value.descriptionKey) : t('meta.default.description')
        const ogLocale = ogLocaleFor(locale.locale)
        const altLocale = ogLocaleFor(locale.locale === 'de' ? 'en' : 'de')
        const title = pageTitle.value
        return [
            { name: 'description', content: description },
            { name: 'robots', content: 'index, follow' },
            { property: 'og:type', content: 'website' },
            { property: 'og:site_name', content: t('meta.siteName') },
            { property: 'og:title', content: title },
            { property: 'og:description', content: description },
            { property: 'og:image', content: OG_IMAGE_URL },
            { property: 'og:url', content: canonical.value },
            { property: 'og:locale', content: ogLocale },
            { property: 'og:locale:alternate', content: altLocale },
            { name: 'twitter:card', content: 'summary_large_image' },
            { name: 'twitter:title', content: title },
            { name: 'twitter:description', content: description },
            { name: 'twitter:image', content: OG_IMAGE_URL },
        ]
    })

    const linkTags = computed<LinkTag[]>(() => {
        if (d.value.noindex === true) return []
        const links: LinkTag[] = [{ rel: 'canonical', href: canonical.value }]
        if (d.value.hreflang === true) {
            // hreflang points at the locale-PREFIXED variants (`/en…`,
            // `/de…`) that vite-ssg actually prerenders — each is a real,
            // crawlable URL, not a query-string hint. x-default → English.
            const suffix = localeSuffix.value
            links.push(
                { rel: 'alternate', hreflang: 'en', href: localizedUrl('en', suffix), type: 'text/html' },
                { rel: 'alternate', hreflang: 'de', href: localizedUrl('de', suffix), type: 'text/html' },
                { rel: 'alternate', hreflang: 'x-default', href: localizedUrl('en', suffix), type: 'text/html' },
            )
        }
        return links
    })

    // One reactive getter for the WHOLE head input (officially supported by
    // unhead v2) — per-key `ResolvableArray` typings reject both bare getters
    // and ComputedRefs of our tag shapes, while the whole-object form infers
    // the items structurally and re-resolves on locale/route changes.
    useHead(() => ({
        title: pageTitle.value,
        htmlAttrs: { lang: locale.locale },
        meta: metaTags.value,
        link: linkTags.value,
    }))
}
