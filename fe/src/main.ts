// SSR polyfills — must run before any Vuetify import. Vuetify and a few
// composables touch browser-only globals at module-eval time; during the
// vite-ssg prerender (happy-dom in Node) these are missing or throw, so
// we stub the ones the public pages exercise. Client builds never hit
// these branches (the globals already exist).
if (typeof globalThis.ResizeObserver === 'undefined') {
    globalThis.ResizeObserver = class {
        observe(): void {}
        unobserve(): void {}
        disconnect(): void {}
    }
}
if (typeof window !== 'undefined' && typeof window.scrollTo !== 'function') {
    // happy-dom throws "Not implemented" for scrollTo; a no-op keeps the
    // router's scrollBehavior from crashing the prerender.
    window.scrollTo = () => {}
}

import { VueQueryPlugin } from '@tanstack/vue-query'
import { createPinia } from 'pinia'
import { ViteSSG } from 'vite-ssg'
import { createVuetify, type IconOptions } from 'vuetify'
import 'vuetify/styles'

import { queryClient } from './api/queryClient'
import App from './App.vue'
import SmartIcon from './components/common/SmartIcon.vue'
import { vuetifyDefaults, vuetifyTheme } from './design-system'
// Tokens.css first so the `:root` palette is applied before Vuetify's
// own styles paint — prevents a flash of the un-themed blue defaults.
import './design-system/tokens.css'
import './design-system/transitions.css'
import { createAppI18n, i18n } from './i18n'
import { installApiSession } from './plugins/apiSession'
import { registerGuards, routes } from './router'
import { installChunkReload } from './router/chunkReload'
import { useLocaleStore } from './stores/locale'

// Vuetify's IconSet#component is typed as `JSXComponent<IconProps>` (a class
// constructor or FunctionalComponent with the precise `IconProps` shape). SFCs
// compile to a wider `DefineComponent` type that is structurally compatible at
// runtime but rejected at compile time under `exactOptionalPropertyTypes`. We
// resolve the slot through Vuetify's public `IconOptions` type to keep the cast
// honest (we only widen here, not at the call site).
type IconSetComponent = NonNullable<IconOptions['sets']>[string]['component']
const smartIconComponent = SmartIcon as unknown as IconSetComponent

// `ViteSSG` replaces `createApp().mount()`. It owns the router (built from
// `routes`) and an `@unhead/vue` head instance, so `useHead` works in the
// `usePageMeta` composable on both server + client. We install the rest —
// Pinia, i18n, Vuetify, VueQuery — in the setup callback, which runs in
// BOTH the prerender (SSR) and the browser. Side effects that only make
// sense in a real browser (session hydrate, chunk-reload, the 401 sign-in
// redirect) are gated on `!import.meta.env.SSR`.
export const createApp = ViteSSG(
    App,
    { routes, base: import.meta.env.BASE_URL },
    ({ app, router, initialState, onSSRAppRendered }) => {
        // A fresh Pinia per render so prerendered pages never leak state
        // into one another (createPinia is a factory — safe to call here).
        const pinia = createPinia()

        // Same isolation for i18n: the prerender runs the locale variants
        // concurrently in one process, so a shared instance would race on
        // `locale`. The browser keeps the module singleton (api/client.ts
        // translates errors through it).
        const i18nInstance = import.meta.env.SSR ? createAppI18n() : i18n

        const vuetify = createVuetify({
            // `ssr: true` makes Vuetify emit markup without reaching for
            // `window` during the prerender; on the client it hydrates.
            ssr: true,
            theme: vuetifyTheme,
            defaults: vuetifyDefaults,
            icons: {
                defaultSet: 'smart',
                aliases: {},
                sets: { smart: { component: smartIconComponent } },
            },
        })

        app.use(pinia)
        // NB: do NOT `app.use(router)` here — ViteSSG installs the router
        // itself immediately after this callback returns. Installing it
        // again triggers Vue's "Plugin has already been applied" warning.
        // We still receive the `router` instance to attach guards + the
        // API session redirect / chunk-reload below, which don't need install.
        app.use(i18nInstance)
        app.use(vuetify)
        app.use(VueQueryPlugin, { queryClient })

        // Persist / restore Pinia across the SSR → client boundary.
        if (import.meta.env.SSR) {
            onSSRAppRendered(() => {
                initialState['pinia'] = pinia.state.value
            })
        } else {
            pinia.state.value = (initialState['pinia'] as typeof pinia.state.value | undefined) ?? {}
        }

        // Keep i18n's active locale in lock-step with the locale store.
        // The store is seeded from the URL `:lang` param by the first
        // guard (registered below).
        useLocaleStore().bindToI18n(i18nInstance)

        // Guards: locale-seeding runs on server + client; the
        // session/family/admin guards short-circuit under SSR (see
        // `registerGuards`).
        registerGuards(router)

        // Session hooks for the API client on both paths; the sign-in
        // redirect only makes sense in the browser.
        installApiSession(
            import.meta.env.SSR
                ? () => {}
                : (to) => {
                      void router.replace(to)
                  },
        )

        if (!import.meta.env.SSR) {
            // Auto-reload a tab whose lazy route chunk 404s after a deploy.
            installChunkReload(router)
        }
    },
)
