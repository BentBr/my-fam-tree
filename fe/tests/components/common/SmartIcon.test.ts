import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import SmartIcon from '@/components/common/SmartIcon.vue'

describe('SmartIcon', () => {
    it('renders a lucide svg from a plain icon name', () => {
        const w = mount(SmartIcon, { props: { icon: 'user' } })
        expect(w.html()).toContain('<svg')
    })

    it('strips the mdi- prefix Vuetify defaults inject', () => {
        const w = mount(SmartIcon, { props: { icon: 'mdi-user' } })
        expect(w.html()).toContain('<svg')
    })

    it('strips a lucide- prefix', () => {
        const w = mount(SmartIcon, { props: { icon: 'lucide-user' } })
        expect(w.html()).toContain('<svg')
    })

    it('renders nothing when icon is not a string', () => {
        const w = mount(SmartIcon, { props: { icon: [] as unknown } })
        expect(w.find('svg').exists()).toBe(false)
    })

    it('renders nothing when icon is empty', () => {
        const w = mount(SmartIcon, { props: { icon: '' } })
        expect(w.find('svg').exists()).toBe(false)
    })
})
