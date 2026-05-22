import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import FamilyTree from '@/components/tree/FamilyTree.vue'
import type { TreeInput } from '@/components/tree/layout'

function tree(): TreeInput {
    return {
        nodes: [
            { id: 'a', given_name: 'A', family_name: '1', parent_ids: [], partner_ids: [] },
            { id: 'b', given_name: 'B', family_name: '2', parent_ids: ['a'], partner_ids: [] },
        ],
        parent_edges: [{ a: 'b', b: 'a' }],
        partner_edges: [],
    }
}

describe('FamilyTree', () => {
    it('mounts and renders an SVG tree with nodes + edges', () => {
        const w = mount(FamilyTree, {
            props: { tree: tree(), selectedId: null, centerOnId: null, currentUserId: null },
            global: {
                stubs: {
                    TreeNode: { template: '<g class="tree-node-stub" />' },
                    TreeEdge: { template: '<g class="tree-edge-stub" />' },
                },
            },
        })
        expect(w.find('svg').exists()).toBe(true)
        expect(w.findAll('.tree-node-stub')).toHaveLength(2)
        expect(w.findAll('.tree-edge-stub')).toHaveLength(1)
    })

    it('mounts with centerOnId targeting an existing node', async () => {
        const w = mount(FamilyTree, {
            props: { tree: tree(), selectedId: null, centerOnId: 'b', currentUserId: null },
            attachTo: document.body,
            global: {
                stubs: {
                    TreeNode: { template: '<g class="tree-node-stub" />' },
                    TreeEdge: { template: '<g class="tree-edge-stub" />' },
                },
            },
        })
        expect(w.find('svg').exists()).toBe(true)
        w.unmount()
    })

    it('reacts to centerOnId changes after mount', async () => {
        const w = mount(FamilyTree, {
            props: { tree: tree(), selectedId: null, centerOnId: null, currentUserId: null },
            attachTo: document.body,
            global: {
                stubs: {
                    TreeNode: { template: '<g class="tree-node-stub" />' },
                    TreeEdge: { template: '<g class="tree-edge-stub" />' },
                },
            },
        })
        await w.setProps({ centerOnId: 'a' })
        expect(w.find('svg').exists()).toBe(true)
        w.unmount()
    })

    it('forwards select events from TreeNode', async () => {
        const w = mount(FamilyTree, {
            props: { tree: tree(), selectedId: null, centerOnId: null, currentUserId: null },
            global: {
                stubs: {
                    TreeNode: {
                        template: '<g class="stub" @click="$emit(\'select\', \'a\')" />',
                        emits: ['select'],
                    },
                    TreeEdge: { template: '<g />' },
                },
            },
        })
        await w.find('.stub').trigger('click')
        expect(w.emitted('select')?.[0]).toEqual(['a'])
    })

    it('propagates a hover from one TreeNode into is-related on the related sibling', async () => {
        // Stub each TreeNode as a small group that mirrors the props we
        // care about into the DOM: `data-id` for routing the hover click,
        // `data-hovered`/`data-related`/`data-dimmed` so we can assert
        // what FamilyTree decided to pass back down on the next tick.
        const TreeNodeStub = {
            props: ['node', 'selected', 'isCurrentUser', 'isHovered', 'isRelated', 'isDimmed'],
            emits: ['select', 'hover'],
            template:
                '<g :data-id="node.id" ' +
                ':data-hovered="isHovered" ' +
                ':data-related="isRelated" ' +
                ':data-dimmed="isDimmed" ' +
                '@click="$emit(\'hover\', node.id)" />',
        }
        const TreeEdgeStub = {
            props: ['kind', 'ax', 'ay', 'bx', 'by', 'isHighlighted', 'isDimmed'],
            template: '<g :data-highlighted="isHighlighted" :data-dimmed="isDimmed" />',
        }
        const w = mount(FamilyTree, {
            props: { tree: tree(), selectedId: null, centerOnId: null, currentUserId: null },
            global: {
                stubs: { TreeNode: TreeNodeStub, TreeEdge: TreeEdgeStub },
            },
        })
        // Fixture has `b` as child of `a` — they are directly related. Hover
        // `a` (via the stub's @click), and `b` should land with isRelated.
        await w.find('[data-id="a"]').trigger('click')
        const a = w.find('[data-id="a"]')
        const b = w.find('[data-id="b"]')
        expect(a.attributes('data-hovered')).toBe('true')
        expect(b.attributes('data-related')).toBe('true')
        expect(b.attributes('data-dimmed')).toBe('false')
        // The parent edge between them should be highlighted, not dimmed.
        const edge = w.find('[data-highlighted="true"]')
        expect(edge.exists()).toBe(true)
    })
})
