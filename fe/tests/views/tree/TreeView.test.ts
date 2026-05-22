import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

interface TreePayload {
    nodes: Array<{
        id: string
        given_name: string
        family_name: string
        linked_user_id?: string | null
        parent_ids: string[]
        partner_ids: string[]
    }>
    parent_edges: Array<{ a: string; b: string }>
    partner_edges: Array<{ a: string; b: string }>
}
const treeData = ref<TreePayload | undefined>(undefined)
const treeIsLoading = ref(false)
const treeError = ref<unknown>(null)
const refetch = vi.fn()

vi.mock('@/api/hooks/relationships', () => ({
    useTree: () => ({ data: treeData, isLoading: treeIsLoading, error: treeError, refetch }),
}))
vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import TreeView from '@/views/tree/TreeView.vue'

function makeRouter(): Router {
    return createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/tree', component: { template: '<div />' } }],
    })
}

async function mountTree(query = '') {
    const router = makeRouter()
    await router.push(`/tree${query}`)
    await router.isReady()
    return mount(TreeView, {
        global: {
            plugins: [i18n, router],
            stubs: {
                'v-toolbar': { template: '<div><slot /></div>' },
                'v-toolbar-title': { template: '<div><slot /></div>' },
                'v-spacer': { template: '<div />' },
                'v-btn': {
                    template:
                        '<button type="button" :data-testid="$attrs[\'data-testid\']" @click="(e) => $emit(\'click\', e)"><slot /></button>',
                    emits: ['click'],
                },
                'v-skeleton-loader': { template: '<div class="skeleton" />' },
                'v-alert': { template: '<div :data-testid="$attrs[\'data-testid\']"><slot /></div>' },
                'v-card': { template: '<div><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-card-text': { template: '<div><slot /></div>' },
                'v-card-actions': { template: '<div><slot /></div>' },
                'v-navigation-drawer': {
                    template: '<aside v-if="modelValue"><slot /></aside>',
                    props: ['modelValue', 'location', 'width', 'temporary'],
                },
                FamilyTree: { template: '<div class="ft-stub" />' },
                PersonDetail: { template: '<div class="pd-stub" />' },
                PersonEdit: {
                    template: '<div class="pe-stub" @click="$emit(\'saved\', \'new\')" />',
                    emits: ['saved', 'cancel'],
                    props: ['mode'],
                },
            },
        },
    })
}

describe('TreeView', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        const store: Record<string, string> = {}
        vi.stubGlobal('localStorage', {
            getItem: (k: string) => store[k] ?? null,
            setItem: (k: string, v: string) => {
                store[k] = v
            },
            removeItem: (k: string) => {
                delete store[k]
            },
        })
        treeData.value = undefined
        treeIsLoading.value = false
        treeError.value = null
        refetch.mockReset()
    })

    it('renders the skeleton while loading', async () => {
        treeIsLoading.value = true
        const w = await mountTree()
        expect(w.find('.skeleton').exists()).toBe(true)
    })

    it('renders the error alert on tree error', async () => {
        treeError.value = new Error('no')
        const w = await mountTree()
        expect(w.find('[data-testid="tree-error"]').exists()).toBe(true)
    })

    it('renders the empty state when zero nodes', async () => {
        treeData.value = { nodes: [], parent_edges: [], partner_edges: [] }
        const w = await mountTree()
        expect(w.find('[data-testid="tree-empty"]').exists()).toBe(true)
    })

    it('renders the FamilyTree when nodes exist', async () => {
        treeData.value = {
            nodes: [{ id: 'a', given_name: 'A', family_name: 'X', parent_ids: [], partner_ids: [] }],
            parent_edges: [],
            partner_edges: [],
        }
        const w = await mountTree()
        expect(w.find('.ft-stub').exists()).toBe(true)
    })

    it('clicking add-person opens the create drawer', async () => {
        treeData.value = {
            nodes: [{ id: 'a', given_name: 'A', family_name: 'X', parent_ids: [], partner_ids: [] }],
            parent_edges: [],
            partner_edges: [],
        }
        const w = await mountTree()
        await w.find('[data-testid="tree-add-person"]').trigger('click')
        await flushPromises()
        expect(w.find('.pe-stub').exists()).toBe(true)
    })

    it('clicking empty-state CTA opens the create drawer', async () => {
        treeData.value = { nodes: [], parent_edges: [], partner_edges: [] }
        const w = await mountTree()
        await w.find('[data-testid="tree-empty-cta"]').trigger('click')
        await flushPromises()
        expect(w.find('.pe-stub').exists()).toBe(true)
    })

    it('saved event closes the drawer and refetches', async () => {
        treeData.value = { nodes: [], parent_edges: [], partner_edges: [] }
        const w = await mountTree()
        await w.find('[data-testid="tree-empty-cta"]').trigger('click')
        await flushPromises()
        await w.find('.pe-stub').trigger('click') // emit saved
        await flushPromises()
        expect(refetch).toHaveBeenCalled()
    })
})
