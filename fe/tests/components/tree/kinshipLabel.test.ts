import { afterEach, describe, expect, it } from 'vitest'

import type { KinshipRelation } from '@/api/types'
import { kinshipLabel } from '@/components/tree/kinshipLabel'
import { i18n } from '@/i18n'
import de from '@/i18n/de/app.json'
import en from '@/i18n/en/app.json'

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

function marriage(ended_on: string | null, end_reason: string | null): Partial<KinshipRelation> {
    return { partnership: { kind: 'marriage', ended_on, end_reason } }
}

function label(r: KinshipRelation, gender: string, locale: 'de' | 'en'): string {
    i18n.global.locale.value = locale
    return kinshipLabel(r, gender, (key, named) => i18n.global.t(key, named))
}

/** Dotted leaf keys of a message subtree. */
function leafKeys(node: unknown, prefix = ''): string[] {
    if (typeof node !== 'object' || node === null) return [prefix]
    return Object.entries(node).flatMap(([key, value]) => leafKeys(value, `${prefix}.${key}`))
}

afterEach(() => {
    i18n.global.locale.value = 'en'
})

describe('kinshipLabel de', () => {
    it.each([
        [rel({ up: 1, down: 0 }), 'male', 'Vater'],
        [rel({ up: 1, down: 0 }), 'female', 'Mutter'],
        [rel({ up: 1, down: 0 }), 'diverse', 'Elternteil'],
        [rel({ up: 1, down: 0 }), '', 'Elternteil'],
        [rel({ up: 2, down: 0 }), 'male', 'Großvater'],
        [rel({ up: 3, down: 0 }), 'female', 'Urgroßmutter'],
        [rel({ up: 4, down: 0 }), 'female', 'Ururgroßmutter'],
        [rel({ up: 0, down: 1 }), 'female', 'Tochter'],
        [rel({ up: 0, down: 2 }), 'female', 'Enkelin'],
        [rel({ up: 0, down: 3 }), 'male', 'Urenkel'],
        [rel({ up: 1, down: 1 }), 'male', 'Bruder'],
        [rel({ up: 1, down: 1 }), '', 'Geschwister'],
        [rel({ up: 1, down: 1, half: true }), 'male', 'Halbbruder'],
        [rel({ up: 1, down: 1, step: true }), 'female', 'Stiefschwester'],
        [rel({ up: 1, down: 0, step: true }), 'female', 'Stiefmutter'],
        [rel({ up: 0, down: 1, step: true }), 'male', 'Stiefsohn'],
        [rel({ up: 2, down: 1 }), 'female', 'Tante'],
        [rel({ up: 3, down: 1 }), 'female', 'Großtante'],
        [rel({ up: 4, down: 1 }), 'female', 'Urgroßtante'],
        [rel({ up: 6, down: 1 }), 'female', 'Urururgroßtante'],
        [rel({ up: 1, down: 2 }), 'female', 'Nichte'],
        [rel({ up: 1, down: 3 }), 'male', 'Großneffe'],
        [rel({ up: 1, down: 4 }), 'male', 'Urgroßneffe'],
        [rel({ up: 2, down: 2 }), 'male', 'Cousin 1. Grades'],
        [rel({ up: 3, down: 3 }), 'female', 'Cousine 2. Grades'],
        [rel({ up: 6, down: 6 }), 'male', 'Cousin 5. Grades'],
        [rel({ up: 3, down: 2 }), 'male', 'Cousin 1. Grades, 1-fach entfernt'],
        [rel({ up: 3, down: 5 }), 'female', 'Cousine 2. Grades, 2-fach entfernt'],
        // A compound prefix lowercases only the first letter of the base.
        [rel({ up: 2, down: 2, step: true }), 'male', 'Stiefcousin 1. Grades'],
        [rel({ up: 2, down: 2, step: true }), 'female', 'Stiefcousine 1. Grades'],
        [rel({ up: 3, down: 2, step: true }), 'male', 'Stiefcousin 1. Grades, 1-fach entfernt'],
        // Neutral forms: the prefix goes on every alternative, not only the first.
        [rel({ up: 2, down: 1, step: true }), '', 'Stiefonkel/Stieftante'],
        [rel({ up: 1, down: 2, step: true }), '', 'Stiefneffe/Stiefnichte'],
        [rel({ up: 4, down: 1 }), '', 'Urgroßonkel/-tante'],
        [rel({ up: 1, down: 0, in_law: 'via_own_partner' }), 'female', 'Schwiegermutter'],
        // A step relation across a partnership keeps its step prefix.
        [
            rel({ up: 1, down: 0, step: true, in_law: 'via_own_partner' }),
            'male',
            'Stiefvater des Partners/der Partnerin',
        ],
        [rel({ up: 0, down: 1, step: true, in_law: 'via_relatives_partner' }), 'female', 'Stieftochter (angeheiratet)'],
        [rel({ up: 0, down: 1, in_law: 'via_relatives_partner' }), 'male', 'Schwiegersohn'],
        [rel({ up: 1, down: 1, in_law: 'via_own_partner' }), 'male', 'Schwager'],
        [rel({ up: 1, down: 1, in_law: 'via_relatives_partner' }), 'female', 'Schwägerin'],
        // A relative's partner vs. a relative of the partner.
        [rel({ up: 2, down: 1, in_law: 'via_relatives_partner' }), 'male', 'Onkel (angeheiratet)'],
        [rel({ up: 2, down: 1, in_law: 'via_own_partner' }), 'female', 'Tante des Partners/der Partnerin'],
        [rel({ up: 2, down: 2, in_law: 'via_own_partner' }), 'male', 'Cousin 1. Grades des Partners/der Partnerin'],
        [rel(marriage(null, null)), 'male', 'Ehemann'],
        [rel(marriage('2020-01-01', 'divorce')), 'female', 'Ex-Ehefrau'],
        [rel(marriage('2020-01-01', 'separation')), 'male', 'Ex-Ehemann'],
        // An end reason without a date still means ended.
        [rel(marriage(null, 'divorce')), 'female', 'Ex-Ehefrau'],
        [rel(marriage('2020-01-01', 'death')), 'male', 'Ehemann (verwitwet)'],
        [rel({ partnership: { kind: 'partnership', ended_on: null, end_reason: null } }), 'female', 'Partnerin'],
        [rel({ partnership: { kind: 'civil_union', ended_on: null, end_reason: null } }), '', 'Partner:in'],
    ])('%j / %s -> %s', (r, gender, expected) => {
        expect(label(r, gender, 'de')).toBe(expected)
    })
})

describe('kinshipLabel en', () => {
    it.each([
        [rel({ up: 2, down: 0 }), 'male', 'grandfather'],
        [rel({ up: 3, down: 0 }), 'male', 'great-grandfather'],
        [rel({ up: 5, down: 0 }), 'female', 'great-great-great-grandmother'],
        [rel({ up: 3, down: 1 }), 'female', 'great-aunt'],
        [rel({ up: 4, down: 1 }), 'female', 'great-great-aunt'],
        [rel({ up: 1, down: 3 }), 'male', 'great-nephew'],
        [rel({ up: 1, down: 1, half: true }), 'female', 'half-sister'],
        [rel({ up: 1, down: 1, step: true }), 'male', 'step-brother'],
        [rel({ up: 2, down: 2 }), 'male', '1-degree cousin'],
        [rel({ up: 6, down: 6 }), 'male', '5-degree cousin'],
        [rel({ up: 3, down: 5 }), 'female', '2-degree cousin, 2× removed'],
        [rel({ up: 2, down: 2, step: true }), 'male', '1-degree step-cousin'],
        [rel({ up: 3, down: 5, step: true }), 'female', '2-degree step-cousin, 2× removed'],
        [rel({ up: 2, down: 1, step: true }), '', 'step-aunt/step-uncle'],
        [rel({ up: 4, down: 1 }), '', 'great-great-aunt/-uncle'],
        [rel({ up: 1, down: 0, in_law: 'via_own_partner' }), 'female', 'mother-in-law'],
        [rel({ up: 1, down: 0, step: true, in_law: 'via_own_partner' }), 'male', "partner's step-father"],
        [rel({ up: 2, down: 1, in_law: 'via_relatives_partner' }), 'male', 'uncle (by marriage)'],
        [rel({ up: 2, down: 1, in_law: 'via_own_partner' }), 'female', "partner's aunt"],
        [rel(marriage('2020-01-01', 'divorce')), 'male', 'ex-husband'],
        [rel(marriage('2020-01-01', 'death')), 'female', 'wife (widowed)'],
    ])('%j / %s -> %s', (r, gender, expected) => {
        expect(label(r, gender, 'en')).toBe(expected)
    })
})

describe('kinship catalogue', () => {
    it('has the same keys in both locales', () => {
        expect(leafKeys(de.tree.kinship).sort()).toEqual(leafKeys(en.tree.kinship).sort())
    })
})
