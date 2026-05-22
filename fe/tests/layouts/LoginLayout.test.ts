import { config, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it } from 'vitest'
import { createRouter, createMemoryHistory } from 'vue-router'

import LoginLayout from '@/layouts/LoginLayout.vue'

import { i18n } from '@/i18n'

// All v-* components are stubbed since they require Vuetify's full plugin
// init; we only verify the template renders + child layout exists.
config.global.stubs = {}

function makeRouter() {
    return createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/', component: { template: '<div>x</div>' } }],
    })
}

describe('LoginLayout', () => {
    it('mounts and renders the language switcher container', async () => {
        const router = makeRouter()
        await router.push('/')
        await router.isReady()
        const wrapper = mount(LoginLayout, {
            global: {
                plugins: [createPinia(), i18n, router],
                stubs: {
                    'v-main': { template: '<main><slot /></main>' },
                    'v-container': { template: '<div><slot /></div>' },
                    'router-view': { template: '<div />' },
                    LangSwitcher: { template: '<div class="lang-stub" />' },
                },
            },
        })
        expect(wrapper.find('h1').exists()).toBe(true)
        expect(wrapper.find('.lang-stub').exists()).toBe(true)
    })
})
