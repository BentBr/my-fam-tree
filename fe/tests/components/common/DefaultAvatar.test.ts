import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import DefaultAvatar from '@/components/common/DefaultAvatar.vue'

/**
 * Vuetify v-avatar + v-img are full Vuetify components — they require
 * directive registration the test plugin doesn't ship by default. The
 * shape we want to assert (initials math, deterministic colour, image
 * vs initials branching) lives in the script section, so stub the two
 * v-* components and inspect the rendered DOM around them.
 */
const stubs = {
    'v-avatar': {
        template: '<div class="v-avatar" :data-color="color" :data-size="size"><slot /></div>',
        props: ['color', 'size'],
    },
    'v-img': {
        template: '<img class="v-img" :src="src" :alt="alt" />',
        props: ['src', 'alt', 'cover'],
    },
}

describe('DefaultAvatar', () => {
    it('renders the photo when src is a non-empty string', () => {
        const w = mount(DefaultAvatar, {
            global: { stubs },
            props: { src: 'http://example/photo.jpg', name: 'Karin Hoffmann' },
        })
        const img = w.find('img.v-img')
        expect(img.exists()).toBe(true)
        expect(img.attributes('src')).toBe('http://example/photo.jpg')
        // The image branch hides the initials span.
        expect(w.text().trim()).toBe('')
    })

    it('falls back to initials when src is null', () => {
        const w = mount(DefaultAvatar, {
            global: { stubs },
            props: { src: null, name: 'Karin Hoffmann' },
        })
        expect(w.find('img.v-img').exists()).toBe(false)
        // Two words → first letter of each, uppercased.
        expect(w.text().trim()).toBe('KH')
    })

    it('falls back to initials when src is undefined or empty string', () => {
        const wUndef = mount(DefaultAvatar, {
            global: { stubs },
            props: { name: 'Alice' },
        })
        expect(wUndef.text().trim()).toBe('A')

        const wEmpty = mount(DefaultAvatar, {
            global: { stubs },
            props: { src: '', name: 'Bob Smith' },
        })
        expect(wEmpty.text().trim()).toBe('BS')
    })

    it('uses `?` for empty / whitespace names', () => {
        const w = mount(DefaultAvatar, {
            global: { stubs },
            props: { src: null, name: '   ' },
        })
        expect(w.text().trim()).toBe('?')
    })

    it('drops everything after the second token', () => {
        const w = mount(DefaultAvatar, {
            global: { stubs },
            props: { src: null, name: 'Anna Maria Müller-Lüdenscheid' },
        })
        // First letter of the first two whitespace-separated tokens.
        expect(w.text().trim()).toBe('AM')
    })

    it('paints the same colour every time for the same name (deterministic palette)', () => {
        const colourOf = (name: string): string | undefined =>
            mount(DefaultAvatar, { global: { stubs }, props: { src: null, name } })
                .find('.v-avatar')
                .attributes('data-color')

        expect(colourOf('Karin Hoffmann')).toBe(colourOf('Karin Hoffmann'))
        // Different names land on (almost always) different palette slots —
        // a 12-entry palette has enough spread that two unrelated 8-char
        // names collide with very low probability. We assert one specific
        // distinct pair to pin the deterministic mapping.
        expect(colourOf('Karin Hoffmann')).not.toBe(colourOf('Bob Smith'))
    })

    it('omits the v-avatar `color` prop when a photo is shown', () => {
        const w = mount(DefaultAvatar, {
            global: { stubs },
            props: { src: 'http://example/photo.jpg', name: 'Karin Hoffmann' },
        })
        // Image mode: `color` is undefined so v-avatar uses the default
        // (transparent) background. Stub passes `color` through verbatim.
        expect(w.find('.v-avatar').attributes('data-color')).toBeUndefined()
    })

    it('honours the `size` prop on the underlying v-avatar', () => {
        const w = mount(DefaultAvatar, {
            global: { stubs },
            props: { src: null, name: 'Alice', size: 56 },
        })
        expect(w.find('.v-avatar').attributes('data-size')).toBe('56')
    })
})
