import { VueQueryPlugin } from '@tanstack/vue-query'
import { createPinia } from 'pinia'
import { createApp } from 'vue'
import { createVuetify } from 'vuetify'
import 'vuetify/styles'

import App from './App.vue'
import SmartIcon from './components/common/SmartIcon.vue'
import { vuetifyDefaults, vuetifyTheme } from './design-system'
import { i18n } from './i18n'
import { router } from './router'
import { useLocaleStore } from './stores/locale'
import './design-system/transitions.css'

const vuetify = createVuetify({
    theme: vuetifyTheme,
    defaults: vuetifyDefaults,
    icons: {
        defaultSet: 'smart',
        aliases: {},
        sets: { smart: { component: SmartIcon } },
    },
})

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(i18n)
app.use(vuetify)
app.use(VueQueryPlugin)

useLocaleStore().bindToI18n(i18n)

app.mount('#app')
