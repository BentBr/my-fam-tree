import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { defineComponent, h } from 'vue'

import { i18n as appI18n } from '@/i18n'

interface Wrapper<T> {
    result: T
    queryClient: QueryClient
}

// Boilerplate-free harness for composables that touch Pinia + vue-query +
// vue-i18n. Returns the value the composable's `setup()` returned so the test
// can drive mutations / inspect refs. The composable must run inside a Vue
// setup context (otherwise inject() fails), so we mount a tiny Test
// component and capture the return synchronously.
export function makeHookWrapper<T>(setup: () => T): Wrapper<T> {
    const queryClient = new QueryClient({
        defaultOptions: { queries: { retry: 0 }, mutations: { retry: 0 } },
    })
    let captured: T | undefined
    const Test = defineComponent({
        setup() {
            captured = setup()
            return () => h('div')
        },
    })
    mount(Test, {
        global: {
            plugins: [createPinia(), appI18n, [VueQueryPlugin, { queryClient }]],
        },
    })
    if (captured === undefined) throw new Error('setup did not produce a value')
    return { result: captured, queryClient }
}
