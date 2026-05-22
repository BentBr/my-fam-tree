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
})
