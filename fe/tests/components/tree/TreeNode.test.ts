import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import { i18n } from '@/i18n'
import TreeNode from '@/components/tree/TreeNode.vue'
import type { Positioned } from '@/components/tree/layout'

function node(over: Partial<Positioned> = {}): Positioned {
    return {
        id: 'n1',
        given_name: 'Alice',
        family_name: 'Smith',
        birth_date: '1990-01-01',
        death_date: null,
        linked_user_id: null,
        is_favourite_for_me: false,
        x: 10,
        y: 20,
        ...over,
    }
}

// TreeNode uses `useI18n` for the favourite tooltip; every mount needs
// the i18n plugin so `t(...)` doesn't throw. Accepts the same options
// shape as `mount` so existing call sites passing `{ props }` keep
// working — we just merge in `global.plugins`.
type MountOptions = Parameters<typeof mount<typeof TreeNode>>[1]
function mountNode(options: MountOptions): ReturnType<typeof mount<typeof TreeNode>> {
    return mount(TreeNode, { ...options, global: { plugins: [i18n] } })
}

describe('TreeNode', () => {
    it('renders given+family name and the birth date row', () => {
        const w = mountNode({ props: { node: node(), selected: false } })
        expect(w.text()).toContain('Alice')
        expect(w.text()).toContain('Smith')
        expect(w.text()).toContain('* 1990-01-01')
    })

    it('emits select on click', async () => {
        const w = mountNode({ props: { node: node(), selected: false } })
        await w.find('g').trigger('click')
        const events = w.emitted('select')
        expect(events).toBeDefined()
        expect(events?.[0]).toEqual(['n1'])
    })

    it('emits select on Enter and Space keys', async () => {
        const w = mountNode({ props: { node: node(), selected: false } })
        await w.find('g').trigger('keydown.enter')
        await w.find('g').trigger('keydown.space')
        expect(w.emitted('select')?.length).toBe(2)
    })

    it('renders initials, falling back to "?" when names are empty', () => {
        const w = mountNode({
            props: { node: node({ given_name: '', family_name: '' }), selected: false },
        })
        expect(w.find('text').text()).toBe('?')
    })

    it('renders birth and death dates on separate lines (* / † prefixes)', () => {
        const w = mountNode({
            props: {
                node: node({ birth_date: '1900', death_date: '1980' }),
                selected: false,
            },
        })
        const birth = w.find('[data-testid="tree-node-birth"]')
        const death = w.find('[data-testid="tree-node-death"]')
        expect(birth.exists()).toBe(true)
        expect(death.exists()).toBe(true)
        expect(birth.text()).toBe('* 1900')
        expect(death.text()).toBe('† 1980')
        // Death date sits below birth in the layout.
        expect(Number(death.attributes('y'))).toBeGreaterThan(Number(birth.attributes('y')))
    })

    it('omits the death row when only birth_date is set', () => {
        const w = mountNode({
            props: { node: node({ birth_date: '1990-01-01' }), selected: false },
        })
        expect(w.find('[data-testid="tree-node-birth"]').exists()).toBe(true)
        expect(w.find('[data-testid="tree-node-death"]').exists()).toBe(false)
    })

    it('renders no date label when neither is set', () => {
        const w = mountNode({
            props: {
                node: node({ birth_date: null, death_date: null }),
                selected: false,
            },
        })
        // The dates row is omitted entirely (v-if) when there is nothing to
        // render — keeps the card visually balanced without an empty line.
        expect(w.find('.dates').exists()).toBe(false)
    })

    it('emits hover events on mouseenter/mouseleave for the parent canvas to track', async () => {
        // Hover state lives on the canvas (so siblings can react), not on
        // the TreeNode — so the only contract here is: TreeNode emits.
        const w = mountNode({ props: { node: node(), selected: false } })
        await w.find('g').trigger('mouseenter')
        await w.find('g').trigger('mouseleave')
        const events = w.emitted('hover')
        expect(events).toBeDefined()
        expect(events?.[0]).toEqual(['n1'])
        expect(events?.[1]).toEqual([null])
    })

    it('applies the hovered class when the isHovered prop is true', () => {
        const w = mountNode({ props: { node: node(), selected: false, isHovered: true } })
        expect(w.find('g').classes()).toContain('hovered')
    })

    it('does NOT apply hovered when isHovered is false / omitted', () => {
        const w = mountNode({ props: { node: node(), selected: false } })
        expect(w.find('g').classes()).not.toContain('hovered')
    })

    it('applies the related class when the isRelated prop is true', () => {
        const w = mountNode({ props: { node: node(), selected: false, isRelated: true } })
        expect(w.find('g').classes()).toContain('related')
    })

    it('applies the dimmed class when the isDimmed prop is true', () => {
        const w = mountNode({ props: { node: node(), selected: false, isDimmed: true } })
        expect(w.find('g').classes()).toContain('dimmed')
    })

    it('applies the selected class when selected', () => {
        const w = mountNode({ props: { node: node(), selected: true } })
        expect(w.find('g').classes()).toContain('selected')
    })

    it('applies the current-user class when isCurrentUser is true', () => {
        const w = mountNode({
            props: { node: node(), selected: false, isCurrentUser: true },
        })
        expect(w.find('g').classes()).toContain('current-user')
    })

    it('does NOT apply current-user when the prop is false / omitted', () => {
        const w = mountNode({ props: { node: node(), selected: false } })
        expect(w.find('g').classes()).not.toContain('current-user')
    })

    it('applies the deceased class when death_date is set', () => {
        const w = mountNode({
            props: {
                node: node({ death_date: '1980-04-12' }),
                selected: false,
            },
        })
        expect(w.find('g').classes()).toContain('deceased')
    })

    it('does NOT apply deceased when death_date is null', () => {
        const w = mountNode({
            props: { node: node({ death_date: null }), selected: false },
        })
        expect(w.find('g').classes()).not.toContain('deceased')
    })

    // --- Age cell ---
    // Living person: shows full years since birth. We freeze "today" via
    // vi.useFakeTimers so the test is stable across years.

    it('renders current age (full years today minus birth) for a living person', async () => {
        const { vi } = await import('vitest')
        vi.useFakeTimers()
        vi.setSystemTime(new Date(2026, 5, 1)) // 2026-06-01 local
        try {
            const w = mountNode({
                props: { node: node({ birth_date: '1990-01-12' }), selected: false },
            })
            const age = w.find('[data-testid="tree-node-age"]')
            expect(age.exists()).toBe(true)
            // 1990-01-12 → 2026-06-01: birthday already past, full years = 36.
            expect(age.text()).toBe('36')
        } finally {
            vi.useRealTimers()
        }
    })

    it('subtracts a year when the birthday has not happened yet this year', async () => {
        const { vi } = await import('vitest')
        vi.useFakeTimers()
        vi.setSystemTime(new Date(2026, 0, 1)) // 2026-01-01 local
        try {
            const w = mountNode({
                props: { node: node({ birth_date: '1990-06-15' }), selected: false },
            })
            // 2026-01-01 hasn't reached 1990-06-15's anniversary, so age = 35.
            expect(w.find('[data-testid="tree-node-age"]').text()).toBe('35')
        } finally {
            vi.useRealTimers()
        }
    })

    it('shows age at death (with †) for a deceased person', () => {
        const w = mountNode({
            props: { node: node({ birth_date: '1900-05-04', death_date: '1980-04-12' }), selected: false },
        })
        const age = w.find('[data-testid="tree-node-age"]')
        expect(age.exists()).toBe(true)
        // 1900-05-04 → 1980-04-12: birthday not yet reached in 1980 → 79.
        expect(age.text()).toBe('79 (†)')
    })

    it('omits the age cell when birth_date is missing', () => {
        const w = mountNode({
            props: { node: node({ birth_date: null }), selected: false },
        })
        expect(w.find('[data-testid="tree-node-age"]').exists()).toBe(false)
    })

    it('omits the age cell for an unparseable birth_date', () => {
        const w = mountNode({
            props: { node: node({ birth_date: 'not-a-date' }), selected: false },
        })
        expect(w.find('[data-testid="tree-node-age"]').exists()).toBe(false)
    })

    it('accepts a bare YYYY birth_date and computes age from Jan 1', async () => {
        const { vi } = await import('vitest')
        vi.useFakeTimers()
        vi.setSystemTime(new Date(2026, 5, 1)) // 2026-06-01
        try {
            const w = mountNode({
                props: { node: node({ birth_date: '1990' }), selected: false },
            })
            // 1990-01-01 → 2026-06-01: birthday passed → 36.
            expect(w.find('[data-testid="tree-node-age"]').text()).toBe('36')
        } finally {
            vi.useRealTimers()
        }
    })

    // --- Favourite star ---

    it('renders the favourite star with the per-node testid', () => {
        const w = mountNode({ props: { node: node(), selected: false } })
        const star = w.find('[data-testid="tree-node-favourite-n1"]')
        expect(star.exists()).toBe(true)
        // Outline-only when not a favourite.
        expect(star.classes()).not.toContain('filled')
    })

    it('applies the filled class when is_favourite_for_me is true', () => {
        const w = mountNode({
            props: { node: node({ is_favourite_for_me: true }), selected: false },
        })
        expect(w.find('[data-testid="tree-node-favourite-n1"]').classes()).toContain('filled')
    })

    it('emits toggle-favourite with the NEXT state on star click and does NOT emit select', async () => {
        const w = mountNode({ props: { node: node({ is_favourite_for_me: false }), selected: false } })
        await w.find('[data-testid="tree-node-favourite-n1"]').trigger('click')
        const events = w.emitted('toggle-favourite')
        expect(events).toBeDefined()
        // id + next=true (flipping from false → true).
        expect(events?.[0]).toEqual(['n1', true])
        // Star click stops propagation, so the card's select must NOT fire.
        expect(w.emitted('select')).toBeUndefined()
    })

    it('emits toggle-favourite with next=false when already a favourite', async () => {
        const w = mountNode({ props: { node: node({ is_favourite_for_me: true }), selected: false } })
        await w.find('[data-testid="tree-node-favourite-n1"]').trigger('click')
        expect(w.emitted('toggle-favourite')?.[0]).toEqual(['n1', false])
    })
})
