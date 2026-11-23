import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import type { KinshipRelation } from '@/api/types'
import KinshipBanner from '@/components/tree/KinshipBanner.vue'
import { i18n } from '@/i18n'

function rel(partial: Partial<KinshipRelation>): KinshipRelation {
    return {
        up: 0,
        down: 0,
        half: false,
        step: false,
        in_law: null,
        partnership: null,
        via_partnership_id: null,
        path: [],
        ...partial,
    }
}

const AUNT = rel({ up: 2, down: 1, path: ['tom', 'dad', 'grandma', 'maria'] })
const SISTER_IN_LAW = rel({ up: 1, down: 1, in_law: 'via_own_partner', path: ['tom', 'wife', 'mil', 'maria'] })

interface BannerProps {
    selectedCount?: number
    relations?: KinshipRelation[] | null
    loading?: boolean
    failed?: boolean
}

const button = { template: '<button type="button" @click="$emit(\'click\')"><slot /></button>', emits: ['click'] }

function mountBanner(props: BannerProps = {}) {
    return mount(KinshipBanner, {
        props: {
            nameA: 'Tom Kin',
            nameB: 'Maria Kin',
            genderB: 'female',
            selectedCount: 2,
            relations: null,
            loading: false,
            failed: false,
            ...props,
        },
        global: {
            plugins: [i18n],
            stubs: {
                'v-card': { template: '<div><slot /></div>' },
                'v-card-text': { template: '<div><slot /></div>' },
                'v-progress-circular': { template: '<i />' },
                'v-btn': button,
                'v-chip': button,
            },
        },
    })
}

describe('KinshipBanner', () => {
    it('asks for two persons, then for the second', () => {
        expect(mountBanner({ selectedCount: 0 }).get('[data-testid="kinship-hint"]').text()).toBe(
            'Select two persons to compare…',
        )
        expect(mountBanner({ selectedCount: 1 }).get('[data-testid="kinship-hint"]').text()).toBe(
            'Select a second person…',
        )
    })

    it('shows the loading state for a complete pair without a result', () => {
        const w = mountBanner({ loading: true })
        expect(w.find('[data-testid="kinship-loading"]').exists()).toBe(true)
        expect(w.emitted('active-relation')?.at(-1)).toEqual([null])
    })

    it('keeps showing the loading state while a retry is running', () => {
        const w = mountBanner({ loading: true, failed: true })
        expect(w.find('[data-testid="kinship-loading"]').exists()).toBe(true)
        expect(w.find('[data-testid="kinship-error"]').exists()).toBe(false)
    })

    it('never renders an empty card: a pair without result, error or fetch (offline) reads as loading', () => {
        const w = mountBanner({ relations: null, loading: false, failed: false })
        expect(w.find('[data-testid="kinship-loading"]').exists()).toBe(true)
    })

    it('shows the error with a retry action when there is no result', async () => {
        const w = mountBanner({ failed: true })
        expect(w.get('[data-testid="kinship-error"]').text()).toContain('could not be loaded')
        await w.get('[data-testid="kinship-retry"]').trigger('click')
        expect(w.emitted('retry')).toHaveLength(1)
    })

    it('says so when the two persons are not related', () => {
        const w = mountBanner({ relations: [] })
        expect(w.get('[data-testid="kinship-none"]').text()).toBe('No relation found')
        expect(w.find('[data-testid="kinship-sentence"]').exists()).toBe(false)
    })

    it('names B from A’s perspective and offers one chip per relation', async () => {
        const w = mountBanner({ relations: [AUNT, SISTER_IN_LAW] })
        expect(w.get('[data-testid="kinship-sentence"]').text()).toBe("Maria Kin is Tom Kin's aunt")
        expect(w.findAll('[data-testid^="kinship-chip-"]').map((c) => c.text())).toEqual(['aunt', 'sister-in-law'])
        // The first relation is lit by default.
        expect(w.emitted('active-relation')?.at(-1)).toEqual([AUNT])

        await w.get('[data-testid="kinship-chip-1"]').trigger('click')
        expect(w.get('[data-testid="kinship-sentence"]').text()).toBe("Maria Kin is Tom Kin's sister-in-law")
        expect(w.emitted('active-relation')?.at(-1)).toEqual([SISTER_IN_LAW])
    })

    it('shows no chips for a single relation', () => {
        const w = mountBanner({ relations: [AUNT] })
        expect(w.findAll('[data-testid^="kinship-chip-"]')).toHaveLength(0)
    })

    it('falls back to the first relation when the result changes', async () => {
        const w = mountBanner({ relations: [AUNT, SISTER_IN_LAW] })
        await w.get('[data-testid="kinship-chip-1"]').trigger('click')
        await w.setProps({ relations: [SISTER_IN_LAW, AUNT] })
        expect(w.get('[data-testid="kinship-sentence"]').text()).toBe("Maria Kin is Tom Kin's sister-in-law")
        expect(w.emitted('active-relation')?.at(-1)).toEqual([SISTER_IN_LAW])
    })
})
