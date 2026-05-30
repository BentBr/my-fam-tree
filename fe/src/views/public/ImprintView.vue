<script setup lang="ts">
/**
 * `/imprint` — legal disclosure (DACH operator block).
 *
 * Content + responsible party + contact details come from i18n (see
 * `public.imprint.*`). Email is `hello@my-fam-tree.eu`; phone is the
 * operator's existing line.
 *
 * Not indexable: the `<meta name="robots" content="noindex,nofollow">`
 * meta is set via `useHead` below; the nginx config also injects an
 * `X-Robots-Tag: noindex,nofollow` header for defence in depth.
 */
import { useI18n } from 'vue-i18n'
import { useHead } from '@unhead/vue'

import { useLocaleStore } from '@/stores/locale'

const { t } = useI18n()
const locale = useLocaleStore()

useHead({
    title: () => `${t('public.imprint.title')} — My Family Tree`,
    htmlAttrs: { lang: () => locale.locale },
    meta: [{ name: 'robots', content: 'noindex, nofollow' }],
})

const email = 'hello@my-fam-tree.eu'
const phone = '+49 (0) 172 511 2820'
</script>

<template>
    <article class="legal" data-testid="public-imprint">
        <h1 class="legal__title display">{{ t('public.imprint.title') }}</h1>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.imprint.sections.contact.heading') }}</h2>
            <dl class="legal__dl">
                <dt>{{ t('public.imprint.sections.contact.email') }}</dt>
                <dd>
                    <a :href="`mailto:${email}`">{{ email }}</a>
                </dd>
                <dt>{{ t('public.imprint.sections.contact.phone') }}</dt>
                <dd>{{ phone }}</dd>
            </dl>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.imprint.sections.responsible.heading') }}</h2>
            <p>{{ t('public.imprint.sections.responsible.name') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.imprint.sections.disclaimer.heading') }}</h2>
            <p>{{ t('public.imprint.sections.disclaimer.body') }}</p>
        </section>

        <section class="legal__section">
            <h2 class="legal__heading">{{ t('public.imprint.sections.linkLiability.heading') }}</h2>
            <p>{{ t('public.imprint.sections.linkLiability.body') }}</p>
        </section>

        <p class="legal__footer">{{ t('public.imprint.footerNote') }}</p>
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
.legal__dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 18px;
    row-gap: 4px;
    margin: 0;
}
.legal__dl dt {
    color: var(--text-3);
    font-weight: 600;
}
.legal__dl dd {
    margin: 0;
    color: var(--text);
}
.legal__dl a {
    color: var(--acc-strong);
    text-decoration: none;
}
.legal__dl a:hover {
    text-decoration: underline;
}
.legal__footer {
    margin-top: 40px;
    color: var(--text-3);
    font-size: 13px;
}
</style>
