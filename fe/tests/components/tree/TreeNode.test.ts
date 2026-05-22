import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

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
        x: 10,
        y: 20,
        ...over,
    }
}

describe('TreeNode', () => {
    it('renders given+family name and the birth date row', () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false } })
        expect(w.text()).toContain('Alice')
        expect(w.text()).toContain('Smith')
        expect(w.text()).toContain('* 1990-01-01')
    })

    it('emits select on click', async () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false } })
        await w.find('g').trigger('click')
        const events = w.emitted('select')
        expect(events).toBeDefined()
        expect(events?.[0]).toEqual(['n1'])
    })

    it('emits select on Enter and Space keys', async () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false } })
        await w.find('g').trigger('keydown.enter')
        await w.find('g').trigger('keydown.space')
        expect(w.emitted('select')?.length).toBe(2)
    })

    it('renders initials, falling back to "?" when names are empty', () => {
        const w = mount(TreeNode, {
            props: { node: node({ given_name: '', family_name: '' }), selected: false },
        })
        expect(w.find('text').text()).toBe('?')
    })

    it('renders birth and death dates on separate lines (* / † prefixes)', () => {
        const w = mount(TreeNode, {
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
        const w = mount(TreeNode, {
            props: { node: node({ birth_date: '1990-01-01' }), selected: false },
        })
        expect(w.find('[data-testid="tree-node-birth"]').exists()).toBe(true)
        expect(w.find('[data-testid="tree-node-death"]').exists()).toBe(false)
    })

    it('renders no date label when neither is set', () => {
        const w = mount(TreeNode, {
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
        const w = mount(TreeNode, { props: { node: node(), selected: false } })
        await w.find('g').trigger('mouseenter')
        await w.find('g').trigger('mouseleave')
        const events = w.emitted('hover')
        expect(events).toBeDefined()
        expect(events?.[0]).toEqual(['n1'])
        expect(events?.[1]).toEqual([null])
    })

    it('applies the hovered class when the isHovered prop is true', () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false, isHovered: true } })
        expect(w.find('g').classes()).toContain('hovered')
    })

    it('does NOT apply hovered when isHovered is false / omitted', () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false } })
        expect(w.find('g').classes()).not.toContain('hovered')
    })

    it('applies the related class when the isRelated prop is true', () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false, isRelated: true } })
        expect(w.find('g').classes()).toContain('related')
    })

    it('applies the dimmed class when the isDimmed prop is true', () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false, isDimmed: true } })
        expect(w.find('g').classes()).toContain('dimmed')
    })

    it('applies the selected class when selected', () => {
        const w = mount(TreeNode, { props: { node: node(), selected: true } })
        expect(w.find('g').classes()).toContain('selected')
    })

    it('applies the current-user class when isCurrentUser is true', () => {
        const w = mount(TreeNode, {
            props: { node: node(), selected: false, isCurrentUser: true },
        })
        expect(w.find('g').classes()).toContain('current-user')
    })

    it('does NOT apply current-user when the prop is false / omitted', () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false } })
        expect(w.find('g').classes()).not.toContain('current-user')
    })

    it('applies the deceased class when death_date is set', () => {
        const w = mount(TreeNode, {
            props: {
                node: node({ death_date: '1980-04-12' }),
                selected: false,
            },
        })
        expect(w.find('g').classes()).toContain('deceased')
    })

    it('does NOT apply deceased when death_date is null', () => {
        const w = mount(TreeNode, {
            props: { node: node({ death_date: null }), selected: false },
        })
        expect(w.find('g').classes()).not.toContain('deceased')
    })
})
