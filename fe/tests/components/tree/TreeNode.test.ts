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
    it('renders given+family name and the date label', () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false } })
        expect(w.text()).toContain('Alice')
        expect(w.text()).toContain('Smith')
        expect(w.text()).toContain('1990-01-01')
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

    it('renders a date range when death_date is set', () => {
        const w = mount(TreeNode, {
            props: {
                node: node({ birth_date: '1900', death_date: '1980' }),
                selected: false,
            },
        })
        expect(w.text()).toContain('1900 – 1980')
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

    it('hover/leave toggles the hovered class', async () => {
        const w = mount(TreeNode, { props: { node: node(), selected: false } })
        await w.find('g').trigger('mouseenter')
        expect(w.find('g').classes()).toContain('hovered')
        await w.find('g').trigger('mouseleave')
        expect(w.find('g').classes()).not.toContain('hovered')
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
