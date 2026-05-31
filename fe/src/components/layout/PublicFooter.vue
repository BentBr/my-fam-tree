<script setup lang="ts">
/**
 * Public-site footer.
 *
 * Carries the two legal-page links (imprint + data policy), the
 * © tagline, and the resolved locale label. Only mounted by
 * `PublicLayout` — the authenticated layouts don't render this
 * because their chrome (sidebar + AppBar) already covers the
 * relevant affordances.
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { useLocaleStore } from '@/stores/locale'

// Stack icons next to the brand names in the tagline. SVG assets live
// in src/assets/ so Vite hashes the URLs (cache-busts on swap) and the
// img-loader picks them up off the year-long immutable /assets/* cache.
//   Ferris  — CC0 from rustacean.net (official Rust mascot).
//   Vue.js  — two-triangle official logo, redrawn from the brand guide.
import ferrisIcon from '@/assets/brand/ferris.svg'
import vueIcon from '@/assets/brand/vue.svg'

const { t } = useI18n()
const locale = useLocaleStore()
const localeLabel = computed(() => (locale.locale === 'de' ? 'Deutsch' : 'English'))
</script>

<template>
    <footer class="public-footer" data-testid="public-footer">
        <div class="public-footer__inner">
            <!-- Tagline is split around the two stack brand-icons so
                 each icon sits inline with its name. We avoid v-html /
                 i18n-component-interpolation for the simple two-icon
                 case; instead the i18n strings split at the icon
                 boundary (prefix + mid). The brand NAMES ("Rust",
                 "Vue.js") are hardcoded — same in every language. -->
            <span class="public-footer__tagline" data-testid="public-footer-tagline">
                {{ t('public.footer.taglinePrefix') }}
                <img :src="ferrisIcon" class="public-footer__stack-icon public-footer__stack-icon--ferris" alt="" />
                <span class="public-footer__stack-name">Rust</span>
                {{ t('public.footer.taglineMid') }}
                <img :src="vueIcon" class="public-footer__stack-icon" alt="" />
                <span class="public-footer__stack-name">Vue.js</span>.
            </span>
            <nav class="public-footer__links" aria-label="legal">
                <RouterLink to="/imprint" data-testid="footer-imprint">
                    {{ t('public.footer.links.imprint') }}
                </RouterLink>
                <RouterLink to="/data-policy" data-testid="footer-data-policy">
                    {{ t('public.footer.links.dataPolicy') }}
                </RouterLink>
                <!-- GH source link — heart emoji + external repo URL. The
                     `aria-label` is more descriptive than the visible
                     content for screen-reader users (the heart on its
                     own would read as "red heart"). `target="_blank"` +
                     `rel="noopener"` per the standard external-link
                     hardening; `noreferrer` is overkill for a public
                     repo URL. -->
                <a
                    href="https://github.com/BentBr/my-fam-tree"
                    target="_blank"
                    rel="noopener"
                    class="public-footer__source"
                    :aria-label="t('public.footer.links.sourceAria')"
                    data-testid="footer-source"
                >
                    {{ t('public.footer.links.sourceLabel') }}
                    <span aria-hidden="true">❤</span>
                </a>
                <span class="public-footer__locale"> {{ t('public.footer.links.language') }}: {{ localeLabel }} </span>
            </nav>
        </div>
    </footer>
</template>

<style scoped>
.public-footer {
    border-top: 1px solid var(--border);
    padding-block: 24px;
    padding-inline: clamp(16px, 4vw, 32px);
    color: var(--text-3);
    font-size: 13px;
}
.public-footer__inner {
    max-width: 1200px;
    margin-inline: auto;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
}
.public-footer__links {
    display: inline-flex;
    align-items: center;
    gap: 22px;
}
.public-footer__links a {
    color: var(--text-2);
    text-decoration: none;
    border-bottom: 1px dashed transparent;
}
.public-footer__links a:hover {
    color: var(--acc-strong);
    border-bottom-color: currentColor;
}
.public-footer__locale {
    color: var(--text-3);
}
.public-footer__tagline {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    line-height: 1.6;
}
.public-footer__stack-icon {
    /* Match the cap height of the surrounding 13 px footer text so
       the icons sit on the baseline like a glyph would. Width
       follows the asset's intrinsic aspect ratio. */
    height: 1.2em;
    width: auto;
    vertical-align: middle;
    flex-shrink: 0;
    display: inline-block;
}
.public-footer__stack-icon--ferris {
    /* Ferris is wider than tall (3:2 source). Cap the height a touch
       LARGER than Vue's so the silhouette reads at footer scale —
       Ferris's distinctive features (eyes + claws) need a few extra
       px to resolve. */
    height: 1.6em;
}
.public-footer__stack-name {
    /* Keep "Rust" / "Vue.js" inline with their icons; the parent
       flex's gap handles spacing between icon + name + connectives. */
    color: inherit;
}
</style>
