import { VueQueryPlugin } from '@tanstack/vue-query'
import { createPinia } from 'pinia'
import { createApp } from 'vue'
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
import { i18n } from './i18n'
import { router } from './router'
import { useLocaleStore } from './stores/locale'

// Vuetify's IconSet#component is typed as `JSXComponent<IconProps>` (a class
// constructor or FunctionalComponent with the precise `IconProps` shape). SFCs
// compile to a wider `DefineComponent` type that is structurally compatible at
// runtime but rejected at compile time under `exactOptionalPropertyTypes`. We
// resolve the slot through Vuetify's public `IconOptions` type to keep the cast
// honest (we only widen here, not at the call site).
type IconSetComponent = NonNullable<IconOptions['sets']>[string]['component']
const smartIconComponent = SmartIcon as unknown as IconSetComponent

const vuetify = createVuetify({
    theme: vuetifyTheme,
    defaults: vuetifyDefaults,
    icons: {
        defaultSet: 'smart',
        aliases: {},
        sets: { smart: { component: smartIconComponent } },
    },
})

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(i18n)
app.use(vuetify)
app.use(VueQueryPlugin, { queryClient })

useLocaleStore().bindToI18n(i18n)

app.mount('#app')
