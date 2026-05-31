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

const { t } = useI18n()
const locale = useLocaleStore()
const localeLabel = computed(() => (locale.locale === 'de' ? 'Deutsch' : 'English'))
</script>

<template>
    <footer class="public-footer" data-testid="public-footer">
        <div class="public-footer__inner">
            <span class="public-footer__tagline">{{ t('public.footer.tagline') }}</span>
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
</style>
