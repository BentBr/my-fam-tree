<script setup lang="ts">
/**
 * `/data-policy` — privacy policy.
 *
 * Distinguishes the public website (no data, no cookies, no tracking)
 * from the authenticated app (data you opt into providing, one
 * strictly-necessary authentication cookie pair). Body copy lives in
 * `i18n/{en,de}.json` under `public.dataPolicy.*`.
 *
 * Not indexable: the `<meta name="robots" content="noindex,nofollow">`
 * meta is set via `useHead` below; the nginx config also injects an
 * `X-Robots-Tag: noindex,nofollow` header for defence in depth.
 */
import { useI18n } from 'vue-i18n'
import { useHead } from '@unhead/vue'

import { useLocaleStore } from '@/stores/locale'

const { t, tm, rt } = useI18n()
const locale = useLocaleStore()

useHead({
    title: () => `${t('public.dataPolicy.title')} — My Family Tree`,
    htmlAttrs: { lang: () => locale.locale },
    meta: [{ name: 'robots', content: 'noindex, nofollow' }],
})

// `tm()` returns the raw catalogue value for a key — we use it to read
// the GDPR-rights array as a list, then `rt()` translates each entry
// through the same interpolation pipeline as `t()`.
const rights = tm('public.dataPolicy.sections.rights.items') as string[]
</script>

<template>
    <article class="legal" data-testid="public-data-policy">
        <h1 class="legal__title display">{{ t('public.dataPolicy.title') }}</h1>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.intro.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.intro.body') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.publicSite.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.publicSite.body') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.whatWeStore.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.whatWeStore.body') }}</p>
            <p>{{ t('public.dataPolicy.sections.whatWeStore.deletion') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.cookies.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.cookies.body') }}</p>
            <ul class="legal__cookies">
                <li>{{ t('public.dataPolicy.sections.cookies.access') }}</li>
                <li>{{ t('public.dataPolicy.sections.cookies.refresh') }}</li>
            </ul>
            <p>{{ t('public.dataPolicy.sections.cookies.rationale') }}</p>
            <p>{{ t('public.dataPolicy.sections.cookies.noOthers') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.noAnalytics.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.noAnalytics.body') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.rights.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.rights.lead') }}</p>
            <ul>
                <li v-for="(item, i) in rights" :key="i">{{ rt(item) }}</li>
            </ul>
            <p>{{ t('public.dataPolicy.sections.rights.trail') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.security.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.security.body') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.changes.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.changes.body') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.contact.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.contact.body') }}</p>
        </section>

        <p class="legal__footer">{{ t('public.dataPolicy.lastUpdated') }}</p>
    </article>
</template>

<style scoped>
.legal {
    max-width: 72ch;
    margin-inline: auto;
    color: var(--text-2);
    font-size: 16px;
    line-height: 1.65;
}
.legal__title {
    font-size: clamp(32px, 4.5vw, 48px);
    font-weight: 700;
    color: var(--text);
    margin: 0 0 32px;
}
.legal__section {
    margin-bottom: 28px;
}
.legal__heading {
    font-size: 20px;
    font-weight: 700;
    color: var(--text);
    margin: 0 0 8px;
}
.legal__cookies {
    list-style: none;
    padding: 0;
    margin: 16px 0;
}
.legal__cookies li {
    border-left: 3px solid var(--acc);
    padding: 4px 0 4px 14px;
    margin-block: 8px;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 14px;
}
.legal__footer {
    margin-top: 40px;
    color: var(--text-3);
    font-size: 13px;
}
</style>
