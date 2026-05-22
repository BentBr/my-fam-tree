import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import LucideIcon from '@/components/common/LucideIcon.vue'

describe('LucideIcon', () => {
    it('renders a lucide component for a known icon name', () => {
        const w = mount(LucideIcon, { props: { name: 'user' } })
        // The resolved component renders an <svg> on the host DOM.
        expect(w.html()).toContain('<svg')
    })

    it('converts kebab-case → PascalCase for multi-word icons', () => {
        const w = mount(LucideIcon, { props: { name: 'user-plus' } })
        expect(w.html()).toContain('<svg')
    })

    it('passes size + strokeWidth + color through', () => {
        const w = mount(LucideIcon, {
            props: { name: 'user', size: 32, strokeWidth: 2.5, color: 'red' },
        })
        const html = w.html()
        expect(html).toContain('width="32"')
        expect(html).toContain('stroke-width="2.5"')
    })

    it('renders nothing when icon name is unknown', () => {
        const w = mount(LucideIcon, { props: { name: 'this-does-not-exist-foo-bar' } })
        // v-if=null branch yields a comment placeholder; no svg appears.
        expect(w.find('svg').exists()).toBe(false)
    })
})
