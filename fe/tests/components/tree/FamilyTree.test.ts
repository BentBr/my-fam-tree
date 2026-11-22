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

    it('mounts with centerOnId targeting an existing node', () => {
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

    it('in highlight mode a node click PINS (no select emit) instead of opening the drawer', async () => {
        const w = mount(FamilyTree, {
            props: { tree: tree(), selectedId: null, centerOnId: null, currentUserId: null, highlightMode: true },
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
        // Highlight mode swallows the select (it pins the lineage instead).
        expect(w.emitted('select')).toBeUndefined()
    })

    it('mounts with currentUserId targeting a linked node (initial focus path)', () => {
        // Linked-user-on-canvas branch: `onMounted` resolves a node center
        // and pans to it at the focus scale instead of fitting the whole
        // tree. The component shouldn't throw or fail to render the SVG.
        const w = mount(FamilyTree, {
            props: {
                tree: {
                    nodes: [
                        {
                            id: 'a',
                            given_name: 'A',
                            family_name: '1',
                            linked_user_id: 'u1',
                            parent_ids: [],
                            partner_ids: [],
                        },
                        { id: 'b', given_name: 'B', family_name: '2', parent_ids: ['a'], partner_ids: [] },
                    ],
                    parent_edges: [{ a: 'b', b: 'a' }],
                    partner_edges: [],
                },
                selectedId: null,
                centerOnId: null,
                currentUserId: 'u1',
            },
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

describe('FamilyTree compare mode', () => {
    // Stubs mirror the props FamilyTree hands down into data-* attributes.
    const NodeStub = {
        props: ['node', 'selected', 'isCurrentUser', 'isHovered', 'isRelated', 'isDimmed', 'compareBadge'],
        emits: ['select', 'hover'],
        template:
            '<g :data-id="node.id" :data-related="isRelated" :data-dimmed="isDimmed" ' +
            ':data-badge="compareBadge ?? \'none\'" ' +
            '@click="$emit(\'select\', node.id)" @mouseenter="$emit(\'hover\', node.id)" />',
    }
    const EdgeStub = {
        props: ['kind', 'ax', 'ay', 'bx', 'by', 'isHighlighted', 'isDimmed'],
        template: '<g class="edge" :data-kind="kind" :data-highlighted="isHighlighted" :data-dimmed="isDimmed" />',
    }

    /** a is the parent of b; c is unrelated. */
    function family(partnerEdges: TreeInput['partner_edges'] = []): TreeInput {
        return {
            nodes: [
                { id: 'a', given_name: 'A', family_name: '1', parent_ids: [], partner_ids: [] },
                { id: 'b', given_name: 'B', family_name: '2', parent_ids: ['a'], partner_ids: [] },
                { id: 'c', given_name: 'C', family_name: '3', parent_ids: [], partner_ids: [] },
            ],
            parent_edges: [{ a: 'b', b: 'a' }],
            partner_edges: partnerEdges,
        }
    }

    /** a and b are partners (no parent link) through every given partnership row. */
    function couple(ids: string[]): TreeInput {
        return {
            nodes: [
                { id: 'a', given_name: 'A', family_name: '1', parent_ids: [], partner_ids: ['b'] },
                { id: 'b', given_name: 'B', family_name: '2', parent_ids: [], partner_ids: ['a'] },
            ],
            parent_edges: [],
            partner_edges: ids.map((id) => ({ a: 'a', b: 'b', id })),
        }
    }

    function mountTree(tree: TreeInput, extra: Record<string, unknown> = {}) {
        return mount(FamilyTree, {
            props: { tree, selectedId: null, centerOnId: null, currentUserId: null, ...extra },
            global: { stubs: { TreeNode: NodeStub, TreeEdge: EdgeStub } },
        })
    }

    it('emits compare-select instead of select while compare mode is on', async () => {
        const w = mountTree(family(), { compareMode: true })
        await w.find('[data-id="b"]').trigger('click')
        expect(w.emitted('compare-select')).toEqual([['b']])
        expect(w.emitted('select')).toBeUndefined()
    })

    it('keeps emitting select while compare mode is off', async () => {
        const w = mountTree(family())
        await w.find('[data-id="b"]').trigger('click')
        expect(w.emitted('select')).toEqual([['b']])
        expect(w.emitted('compare-select')).toBeUndefined()
    })

    it('lights the path and dims everything else', () => {
        const w = mountTree(family(), { highlight: { path: ['b', 'a'], partnershipId: null } })
        const edge = w.find('.edge')
        expect(edge.attributes('data-highlighted')).toBe('true')
        expect(edge.attributes('data-dimmed')).toBe('false')
        expect(w.find('[data-id="a"]').attributes('data-related')).toBe('true')
        expect(w.find('[data-id="a"]').attributes('data-dimmed')).toBe('false')
        expect(w.find('[data-id="c"]').attributes('data-dimmed')).toBe('true')
    })

    it('keeps an off-path person dimmed while hovered: the path wins over hover', async () => {
        const w = mountTree(family(), { highlight: { path: ['b', 'a'], partnershipId: null } })
        await w.find('[data-id="c"]').trigger('mouseenter')
        expect(w.find('[data-id="c"]').attributes('data-dimmed')).toBe('true')
        expect(w.find('[data-id="b"]').attributes('data-dimmed')).toBe('false')
    })

    it('lights only the partnership row the relation crosses', () => {
        const w = mountTree(couple(['p1', 'p2']), { highlight: { path: ['a', 'b'], partnershipId: 'p2' } })
        const edges = w.findAll('[data-kind="partner"]')
        expect(edges).toHaveLength(2)
        expect(edges.map((e) => e.attributes('data-highlighted'))).toEqual(['false', 'true'])
        expect(edges.map((e) => e.attributes('data-dimmed'))).toEqual(['true', 'false'])
    })

    it('does not light a parent link between the two partners of the crossed partnership', () => {
        const tree = family([{ a: 'a', b: 'b', id: 'p1' }])
        const w = mountTree(tree, { highlight: { path: ['a', 'b'], partnershipId: 'p1' } })
        expect(w.find('[data-kind="partner"]').attributes('data-highlighted')).toBe('true')
        expect(w.find('[data-kind="parent"]').attributes('data-highlighted')).toBe('false')
    })

    it('lights no partner edge for a blood relation between two partners', () => {
        const tree = family([{ a: 'a', b: 'b', id: 'p1' }])
        const w = mountTree(tree, { highlight: { path: ['b', 'a'], partnershipId: null } })
        expect(w.find('[data-kind="partner"]').attributes('data-highlighted')).toBe('false')
        expect(w.find('[data-kind="parent"]').attributes('data-highlighted')).toBe('true')
    })

    it('hands the compare slots to the matching nodes', () => {
        const w = mountTree(family(), { compareMode: true, compareAId: 'b', compareBId: 'c' })
        expect(w.find('[data-id="b"]').attributes('data-badge')).toBe('1')
        expect(w.find('[data-id="c"]').attributes('data-badge')).toBe('2')
        expect(w.find('[data-id="a"]').attributes('data-badge')).toBe('none')
    })
})
