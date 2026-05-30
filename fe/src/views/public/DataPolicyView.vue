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
// the rights array as a list, then `rt()` translates each entry through
// the same interpolation pipeline as `t()`.
const rights = tm('public.dataPolicy.sections.rights.items') as string[]

// vue-i18n reserves `@` as the linked-message sigil (`@:other.key`), so
// any literal `@` inside a translation string trips the message
// compiler with "Invalid linked format". The contact email therefore
// stays out of the catalogue and reaches each affected line via a
// named-interpolation slot `{email}`.
const contactEmail = 'hello@my-fam-tree.eu'
</script>

<template>
    <article class="legal" data-testid="public-data-policy">
        <h1 class="legal__title display">{{ t('public.dataPolicy.title') }}</h1>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.introduction.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.introduction.body') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.dataCollection.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.dataCollection.publicBody') }}</p>
            <p>{{ t('public.dataPolicy.sections.dataCollection.appBody') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.cookies.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.cookies.intro') }}</p>
            <ul class="legal__cookies">
                <li>{{ t('public.dataPolicy.sections.cookies.access') }}</li>
                <li>{{ t('public.dataPolicy.sections.cookies.refresh') }}</li>
            </ul>
            <p>{{ t('public.dataPolicy.sections.cookies.rationale') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.analytics.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.analytics.body') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.dataPolicy.sections.rights.heading') }}</h2>
            <p>{{ t('public.dataPolicy.sections.rights.lead') }}</p>
            <ul>
                <li v-for="(item, i) in rights" :key="i">{{ rt(item) }}</li>
            </ul>
            <p>{{ t('public.dataPolicy.sections.rights.trail', { email: contactEmail }) }}</p>
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
            <p>{{ t('public.dataPolicy.sections.contact.body', { email: contactEmail }) }}</p>
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
