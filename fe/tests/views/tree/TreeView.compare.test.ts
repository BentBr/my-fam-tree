import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref, type Ref } from 'vue'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

interface Node {
    id: string
    given_name: string
    family_name: string
    parent_ids: string[]
    partner_ids: string[]
}
interface TreePayload {
    nodes: Node[]
    parent_edges: Array<{ a: string; b: string }>
    partner_edges: Array<{ a: string; b: string }>
}
interface Relation {
    up: number
    down: number
    half: boolean
    step: boolean
    in_law: null
    partnership: null
    via_partnership_id: string | null
    path: string[]
}
interface KinshipPayload {
    person_a: { id: string; gender: string }
    person_b: { id: string; gender: string }
    relations: Relation[]
}

const treeData = ref<TreePayload | undefined>(undefined)
const treeIsFetching = ref(false)
const treeUpdatedAt = ref(0)
const kinshipData = ref<KinshipPayload | undefined>(undefined)
const kinshipIsError = ref(false)
const kinshipIsFetching = ref(false)
const kinshipRefetch = vi.fn()
/** The id refs TreeView handed to `useKinship` — i.e. the pair it would ask about. */
const requested: { a: Ref<string | null> | null; b: Ref<string | null> | null } = { a: null, b: null }

vi.mock('vuetify', () => ({ useDisplay: () => ({ smAndDown: ref(false) }) }))
vi.mock('@/api/hooks/relationships', () => ({
    useTree: () => ({
        data: treeData,
        isLoading: ref(false),
        isFetching: treeIsFetching,
        dataUpdatedAt: treeUpdatedAt,
        error: ref<unknown>(null),
        refetch: vi.fn(),
    }),
    useKinship: (a: Ref<string | null>, b: Ref<string | null>) => {
        requested.a = a
        requested.b = b
        return { data: kinshipData, isFetching: kinshipIsFetching, isError: kinshipIsError, refetch: kinshipRefetch }
    },
}))
vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import type { FamilyId } from '@/types/brand'
import TreeView from '@/views/tree/TreeView.vue'
import { memoryStorage } from '../../helpers/memory-storage'

const A = '00000000-0000-4000-8000-00000000000a'
const B = '00000000-0000-4000-8000-00000000000b'
const C = '00000000-0000-4000-8000-00000000000c'
const UNKNOWN = '00000000-0000-4000-8000-0000000000ff'

function person(id: string, given_name: string): Node {
    return { id, given_name, family_name: 'Kin', parent_ids: [], partner_ids: [] }
}
function treeOf(...nodes: Node[]): TreePayload {
    return { nodes, parent_edges: [], partner_edges: [] }
}
const FAMILY = treeOf(person(A, 'Anna'), person(B, 'Ben'), person(C, 'Cleo'))

function relation(path: string[]): Relation {
    return { up: 1, down: 1, half: false, step: false, in_law: null, partnership: null, via_partnership_id: null, path }
}
function kinshipOf(relations: Relation[]): KinshipPayload {
    return { person_a: { id: A, gender: 'female' }, person_b: { id: B, gender: 'male' }, relations }
}

const button = { template: '<button type="button" @click="$emit(\'click\')"><slot /></button>', emits: ['click'] }
const FamilyTreeStub = {
    name: 'FamilyTreeStub',
    props: ['compareMode', 'compareAId', 'compareBId', 'highlight', 'highlightMode'],
    emits: ['compare-select', 'select'],
    template: '<div class="ft-stub" />',
}

let router: Router

async function mountTree(query = ''): Promise<VueWrapper> {
    router = createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/tree', component: { template: '<div />' } }],
    })
    await router.push(`/tree${query}`)
    await router.isReady()
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const w = mount(TreeView, {
        global: {
            plugins: [i18n, router, [VueQueryPlugin, { queryClient }]],
            stubs: {
                'v-toolbar': { template: '<div><slot /></div>' },
                'v-toolbar-title': { template: '<div><slot /></div>' },
                'v-spacer': { template: '<div />' },
                'v-btn': button,
                'v-chip': button,
                'v-progress-circular': { template: '<i />' },
                'v-skeleton-loader': { template: '<div />' },
                'v-alert': { template: '<div><slot /></div>' },
                'v-card': { template: '<div><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-card-text': { template: '<div><slot /></div>' },
                'v-card-actions': { template: '<div><slot /></div>' },
                'v-navigation-drawer': {
                    template: '<aside v-if="modelValue"><slot /></aside>',
                    props: ['modelValue', 'location', 'width', 'temporary'],
                },
                FamilyTree: FamilyTreeStub,
                PersonDetail: { template: '<div class="pd-stub" />' },
                PersonEdit: { template: '<div class="pe-stub" />', props: ['mode'] },
            },
        },
    })
    await flushPromises()
    return w
}

/** A (re)fetched tree arrives. */
async function treeArrives(tree: TreePayload): Promise<void> {
    treeData.value = tree
    treeUpdatedAt.value += 1
    await flushPromises()
}

function canvas(w: VueWrapper) {
    return w.getComponent(FamilyTreeStub)
}
async function pick(w: VueWrapper, id: string): Promise<void> {
    canvas(w).vm.$emit('compare-select', id)
    await flushPromises()
}
async function click(w: VueWrapper, testid: string): Promise<void> {
    await w.get(`[data-testid="${testid}"]`).trigger('click')
    await flushPromises()
}
function compareParam(): unknown {
    return router.currentRoute.value.query['compare']
}
function asked(): [string | null, string | null] {
    return [requested.a?.value ?? null, requested.b?.value ?? null]
}

describe('TreeView compare mode', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        vi.stubGlobal('localStorage', memoryStorage())
        useActiveFamilyStore().activeFamilyId = 'fam-1' as FamilyId
        treeData.value = FAMILY
        treeIsFetching.value = false
        treeUpdatedAt.value = 1
        kinshipData.value = undefined
        kinshipIsError.value = false
        kinshipIsFetching.value = false
        kinshipRefetch.mockReset()
        requested.a = null
        requested.b = null
    })

    it('switching compare on shows the hint with an empty selection and leaves highlight mode', async () => {
        const w = await mountTree(`?center=${A}`)
        await click(w, 'tree-highlight-mode')
        await click(w, 'tree-compare-mode')
        expect(canvas(w).props('compareMode')).toBe(true)
        expect(canvas(w).props('highlightMode')).toBe(false)
        expect(w.get('[data-testid="kinship-hint"]').text()).toBe('Select two persons to compare…')
        expect(router.currentRoute.value.query).toEqual({})
        expect(w.find('aside').exists()).toBe(false)
    })

    it('builds the pair in the URL: A, then B, a third click replaces B, a selected person is deselected', async () => {
        const w = await mountTree()
        await click(w, 'tree-compare-mode')
        await pick(w, A)
        expect(compareParam()).toBe(A)
        expect(canvas(w).props('compareAId')).toBe(A)
        await pick(w, B)
        expect(compareParam()).toBe(`${A},${B}`)
        expect(asked()).toEqual([A, B])
        await pick(w, C)
        expect(compareParam()).toBe(`${A},${C}`)
        await pick(w, A)
        expect(compareParam()).toBe(C)
        expect(canvas(w).props('compareAId')).toBe(C)
        expect(canvas(w).props('compareBId')).toBeNull()
    })

    it('switching compare off removes the pair, the banner and the highlight', async () => {
        // A center param that survived the deep link goes too: the drawer would
        // not reopen for it, so the URL must not claim it is open.
        const w = await mountTree(`?center=${C}&compare=${A},${B}`)
        kinshipData.value = kinshipOf([relation([A, B])])
        await flushPromises()
        expect(canvas(w).props('highlight')).toEqual({ path: [A, B], partnershipId: null })
        await click(w, 'tree-compare-mode')
        expect(router.currentRoute.value.query).toEqual({})
        expect(w.find('[data-testid="kinship-banner"]').exists()).toBe(false)
        expect(canvas(w).props('compareMode')).toBe(false)
        expect(canvas(w).props('highlight')).toBeNull()
    })

    it('switching highlight on leaves compare mode and removes the pair in the same navigation', async () => {
        const w = await mountTree(`?compare=${A},${B}`)
        await click(w, 'tree-highlight-mode')
        expect(canvas(w).props('compareMode')).toBe(false)
        expect(canvas(w).props('highlightMode')).toBe(true)
        expect(compareParam()).toBeUndefined()
    })

    it('asks about a deep-linked pair only once the tree has loaded', async () => {
        treeData.value = undefined
        treeIsFetching.value = true
        const w = await mountTree(`?compare=${A},${B}`)
        expect(asked()).toEqual([null, null])
        await treeArrives(FAMILY)
        expect(asked()).toEqual([A, B])
        expect(canvas(w).props('compareMode')).toBe(true)
        expect(w.find('[data-testid="kinship-banner"]').exists()).toBe(true)
    })

    it('drops a person that is not in the tree and rewrites the URL', async () => {
        await mountTree(`?compare=${A.toUpperCase()},${UNKNOWN}`)
        expect(asked()).toEqual([A, null])
        expect(compareParam()).toBe(A)
    })

    it('ignores a malformed compare param: no compare mode, param removed', async () => {
        const w = await mountTree('?compare=nope')
        expect(canvas(w).props('compareMode')).toBe(false)
        expect(compareParam()).toBeUndefined()
    })

    it('stops asking the moment the family changes and drops the old pair when the new tree arrives', async () => {
        await mountTree(`?compare=${A},${B}`)
        expect(asked()).toEqual([A, B])
        useActiveFamilyStore().activeFamilyId = 'fam-2' as FamilyId
        // The old tree is still on screen; its persons must not be asked about.
        expect(asked()).toEqual([null, null])
        await treeArrives(treeOf(person(C, 'Cleo')))
        expect(asked()).toEqual([null, null])
        expect(compareParam()).toBeUndefined()
    })

    it('a pair arriving by navigation enters compare mode, leaves highlight mode and closes the drawer', async () => {
        const w = await mountTree()
        await click(w, 'tree-highlight-mode')
        await click(w, 'tree-add-person')
        expect(w.find('aside').exists()).toBe(true)
        await router.push(`/tree?compare=${A},${B}`)
        await flushPromises()
        expect(canvas(w).props('compareMode')).toBe(true)
        expect(canvas(w).props('highlightMode')).toBe(false)
        expect(w.find('aside').exists()).toBe(false)
    })

    it('an emptied compare param empties the selection but keeps compare mode', async () => {
        const w = await mountTree(`?compare=${A},${B}`)
        await router.push('/tree')
        await flushPromises()
        expect(canvas(w).props('compareMode')).toBe(true)
        expect(canvas(w).props('compareAId')).toBeNull()
        expect(w.find('[data-testid="kinship-hint"]').exists()).toBe(true)
    })

    it('Add person leaves compare mode, removes the pair and opens the create drawer', async () => {
        const w = await mountTree(`?compare=${A},${B}`)
        await click(w, 'tree-add-person')
        expect(canvas(w).props('compareMode')).toBe(false)
        expect(compareParam()).toBeUndefined()
        expect(w.find('.pe-stub').exists()).toBe(true)
    })

    it('does not open the person drawer for a center param while comparing', async () => {
        const w = await mountTree(`?center=${C}&compare=${A},${B}`)
        expect(canvas(w).props('compareMode')).toBe(true)
        expect(w.find('aside').exists()).toBe(false)
    })

    it('shows the result and hands the active relation to the canvas', async () => {
        const w = await mountTree(`?compare=${A},${B}`)
        kinshipData.value = kinshipOf([])
        await flushPromises()
        expect(w.find('[data-testid="kinship-none"]').exists()).toBe(true)
        expect(canvas(w).props('highlight')).toBeNull()

        kinshipData.value = kinshipOf([{ ...relation([A, C, B]), via_partnership_id: 'p1' }])
        await flushPromises()
        expect(w.get('[data-testid="kinship-sentence"]').text()).toBe("Ben Kin is Anna Kin's brother")
        expect(canvas(w).props('highlight')).toEqual({ path: [A, C, B], partnershipId: 'p1' })
    })

    it('keeps the cached result when a refetch fails, and shows the error only without one', async () => {
        const w = await mountTree(`?compare=${A},${B}`)
        kinshipData.value = kinshipOf([relation([A, B])])
        kinshipIsError.value = true
        await flushPromises()
        expect(w.find('[data-testid="kinship-sentence"]').exists()).toBe(true)
        expect(w.find('[data-testid="kinship-error"]').exists()).toBe(false)
        expect(canvas(w).props('highlight')).toEqual({ path: [A, B], partnershipId: null })

        kinshipData.value = undefined
        await flushPromises()
        expect(w.find('[data-testid="kinship-error"]').exists()).toBe(true)
        expect(canvas(w).props('highlight')).toBeNull()
        await click(w, 'kinship-retry')
        expect(kinshipRefetch).toHaveBeenCalledTimes(1)
        // While the retry runs the banner shows the spinner, not the stale error.
        kinshipIsFetching.value = true
        await flushPromises()
        expect(w.find('[data-testid="kinship-loading"]').exists()).toBe(true)
        expect(w.find('[data-testid="kinship-error"]').exists()).toBe(false)
    })

    it('shows no banner over the empty-family state', async () => {
        treeData.value = treeOf()
        const w = await mountTree(`?compare=${A},${B}`)
        expect(w.find('[data-testid="tree-empty"]').exists()).toBe(true)
        expect(w.find('[data-testid="kinship-banner"]').exists()).toBe(false)
    })
})
