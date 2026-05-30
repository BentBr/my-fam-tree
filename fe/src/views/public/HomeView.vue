<script setup lang="ts">
/**
 * Public home page — `/`.
 *
 * Layout (top to bottom):
 *   1. Hero: sloth-family image + headline + lede + two CTAs.
 *   2. Feature row: three cards with icon + title + body.
 *   3. Screenshot slot: warm hatched `.ph-img` placeholder until a
 *      tree-view screenshot is dropped at `assets/landing/tree.png`.
 *   4. Footer CTA: bold "Create an account" call.
 *
 * Head metadata is driven by `@unhead/vue`'s `useHead` so it can be
 * picked up by a static-site-generation pass during the build.
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useHead } from '@unhead/vue'

import { useLocaleStore } from '@/stores/locale'

const { t } = useI18n()
const locale = useLocaleStore()

const baseUrl = (import.meta.env['VITE_BASE_URL'] as string | undefined) ?? 'https://my-fam-tree.eu'
const ogLocale = computed(() => (locale.locale === 'de' ? 'de_DE' : 'en_US'))

useHead({
    title: () => t('public.home.hero.title') + ' — My Family Tree',
    htmlAttrs: { lang: () => locale.locale },
    meta: [
        { name: 'description', content: () => t('public.home.hero.lede') },
        { name: 'robots', content: 'index, follow' },
        { property: 'og:type', content: 'website' },
        { property: 'og:title', content: () => t('public.home.hero.title') + ' — My Family Tree' },
        { property: 'og:description', content: () => t('public.home.hero.lede') },
        { property: 'og:image', content: `${baseUrl}/brand/og-1200x630.png` },
        { property: 'og:locale', content: () => ogLocale.value },
        { name: 'twitter:card', content: 'summary_large_image' },
    ],
    link: [
        { rel: 'canonical', href: `${baseUrl}/` },
        { rel: 'alternate', hreflang: 'en', href: `${baseUrl}/` },
        { rel: 'alternate', hreflang: 'de', href: `${baseUrl}/` },
        { rel: 'alternate', hreflang: 'x-default', href: `${baseUrl}/` },
    ],
})

interface Feature {
    key: 'relations' | 'reminders' | 'privacy'
    icon: string
}
const features: Feature[] = [
    { key: 'relations', icon: 'network' },
    { key: 'reminders', icon: 'bell' },
    { key: 'privacy', icon: 'lock' },
]
</script>

<template>
    <article class="home" data-testid="public-home">
        <!-- Hero -->
        <section class="home__hero">
            <div class="home__hero-text">
                <p class="home__eyebrow">{{ t('public.home.hero.eyebrow') }}</p>
                <h1 class="home__title display">{{ t('public.home.hero.title') }}</h1>
                <p class="home__lede">{{ t('public.home.hero.lede') }}</p>
                <div class="home__cta">
                    <v-btn
                        color="primary"
                        variant="flat"
                        size="large"
                        to="/auth/sign-in"
                        data-testid="home-cta-primary"
                    >
                        {{ t('public.home.hero.ctaPrimary') }}
                    </v-btn>
                    <v-btn variant="text" size="large" to="/auth/sign-in" data-testid="home-cta-secondary">
                        {{ t('public.home.hero.ctaSecondary') }}
                    </v-btn>
                </div>
            </div>
            <div class="home__hero-image">
                <img
                    src="/brand/sloth-family-960.webp"
                    width="960"
                    height="640"
                    loading="eager"
                    decoding="async"
                    :alt="t('public.home.hero.imageAlt')"
                />
            </div>
        </section>

        <!-- Features -->
        <section class="home__features">
            <div v-for="f in features" :key="f.key" class="home__feature">
                <v-icon :icon="f.icon" size="32" color="primary" />
                <h2 class="home__feature-title display">{{ t(`public.home.features.${f.key}.title`) }}</h2>
                <p class="home__feature-body">{{ t(`public.home.features.${f.key}.body`) }}</p>
            </div>
        </section>

        <!-- Tree screenshot placeholder — swap to a real PNG once one
             ships under /brand/tree-screenshot.png. -->
        <section class="home__screenshot">
            <div class="ph-img home__screenshot-placeholder">
                {{ t('public.home.screenshot.caption') }}
            </div>
        </section>

        <!-- Final CTA -->
        <section class="home__cta-footer">
            <h2 class="home__cta-title display">{{ t('public.home.cta.title') }}</h2>
            <v-btn color="primary" variant="flat" size="x-large" to="/auth/sign-in" data-testid="home-cta-footer">
                {{ t('public.home.cta.button') }}
            </v-btn>
        </section>
    </article>
</template>

<style scoped>
.home {
    display: flex;
    flex-direction: column;
    gap: clamp(48px, 8vw, 96px);
}

/* ---- Hero ---- */
.home__hero {
    display: grid;
    grid-template-columns: 1fr;
    gap: clamp(24px, 4vw, 48px);
    align-items: center;
}
@media (min-width: 768px) {
    .home__hero {
        grid-template-columns: 1.05fr 1fr;
    }
}
.home__eyebrow {
    color: var(--acc-strong);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 13px;
    margin: 0 0 12px;
}
.home__title {
    font-size: clamp(36px, 5vw, 56px);
    font-weight: 700;
    color: var(--text);
    line-height: 1.1;
    margin: 0 0 16px;
}
.home__lede {
    font-size: clamp(16px, 1.4vw, 18px);
    color: var(--text-2);
    line-height: 1.55;
    margin: 0 0 28px;
    max-width: 56ch;
}
.home__cta {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
}
.home__hero-image img {
    width: 100%;
    height: auto;
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-lg);
    display: block;
}

/* ---- Features ---- */
.home__features {
    display: grid;
    grid-template-columns: 1fr;
    gap: clamp(24px, 3vw, 32px);
}
@media (min-width: 768px) {
    .home__features {
        grid-template-columns: repeat(3, 1fr);
    }
}
.home__feature {
    padding: 24px;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
    display: flex;
    flex-direction: column;
    gap: 12px;
}
.home__feature-title {
    font-size: 18px;
    font-weight: 700;
    color: var(--text);
    margin: 0;
}
.home__feature-body {
    color: var(--text-2);
    font-size: 14.5px;
    line-height: 1.55;
    margin: 0;
}

/* ---- Screenshot slot ---- */
.home__screenshot {
    border-radius: var(--r-lg);
    overflow: hidden;
}
.home__screenshot-placeholder {
    aspect-ratio: 16 / 9;
    width: 100%;
    border-radius: var(--r-lg);
    border: 1px solid var(--border);
    padding: 24px;
    text-align: center;
}

/* ---- Footer CTA ---- */
.home__cta-footer {
    text-align: center;
    padding-block: clamp(32px, 5vw, 56px);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
}
.home__cta-title {
    font-size: clamp(24px, 3.4vw, 36px);
    font-weight: 700;
    color: var(--text);
    margin: 0;
}
</style>
