// Pure mapper: kinship descriptor -> localized label. The descriptor reads
// from A's perspective; the label names B and uses B's gender. The table it
// implements is in `.claude/specs/kinship-comparison.md`.

import type { KinshipRelation } from '@/api/types'

/** The slice of vue-i18n's `t` the mapper needs, so it runs without a component. */
export type Translate = (key: string, named: Record<string, unknown>) => string

type GenderKey = 'm' | 'f' | 'n'
type Partnership = NonNullable<KinshipRelation['partnership']>

const K = 'tree.kinship'

/** `male` / `female` pick the gendered form; every other value is neutral. */
function genderKey(raw: string): GenderKey {
    if (raw === 'male') return 'm'
    if (raw === 'female') return 'f'
    return 'n'
}

function term(t: Translate, name: string, g: GenderKey): string {
    return t(`${K}.${name}.${g}`, {})
}

/**
 * A prefix ending in "-" chains as-is ("great-" + "great-aunt"). A bare prefix
 * forms one compound word, so the base loses its capital ("Ur" + "Großtante").
 * Neutral forms list alternatives with "/": each one gets the prefix
 * ("Stiefonkel/Stieftante"), except a "-tante" style continuation of the
 * first word ("Urgroßonkel/-tante").
 */
function withPrefix(prefix: string, base: string): string {
    const glue = (part: string): string => {
        if (part.startsWith('-')) return part
        return prefix.endsWith('-') ? prefix + part : prefix + part.charAt(0).toLowerCase() + part.slice(1)
    }
    return base.split('/').map(glue).join('/')
}

function repeated(prefix: string, times: number, base: string): string {
    let out = base
    for (let i = 0; i < times; i += 1) out = withPrefix(prefix, out)
    return out
}

/** Both sides are two or more generations below the common ancestor. */
function isCousin(up: number, down: number): boolean {
    return up >= 2 && down >= 2
}

/** Numeric degree / removal names any distance without per-distance keys. */
function cousinLabel(up: number, down: number, g: GenderKey, t: Translate, step: boolean): string {
    const degree = Math.min(up, down) - 1
    const removed = Math.abs(up - down)
    const key = step ? 'stepCousin' : 'cousin'
    if (removed === 0) return t(`${K}.${key}.${g}`, { degree })
    return t(`${K}.${key}Removed.${g}`, { degree, removed })
}

/** Direct line, siblings, aunts / uncles and nieces / nephews — everything but cousins. */
function closeLabel(up: number, down: number, g: GenderKey, t: Translate): string {
    const great = t(`${K}.prefix.great`, {})
    if (down === 0) return up === 1 ? term(t, 'parent', g) : repeated(great, up - 2, term(t, 'grandparent', g))
    if (up === 0) return down === 1 ? term(t, 'child', g) : repeated(great, down - 2, term(t, 'grandchild', g))
    if (up === 1 && down === 1) return term(t, 'sibling', g)
    if (down === 1) return up === 2 ? term(t, 'auntUncle', g) : repeated(great, up - 3, term(t, 'grandAuntUncle', g))
    return down === 2 ? term(t, 'nieceNephew', g) : repeated(great, down - 3, term(t, 'grandNieceNephew', g))
}

/** The relation by generations, with its step / half qualifier. */
function relativeLabel(rel: KinshipRelation, g: GenderKey, t: Translate): string {
    // Cousins carry the step qualifier inside their own phrase ("1-degree step-cousin").
    if (isCousin(rel.up, rel.down)) return cousinLabel(rel.up, rel.down, g, t, rel.step)
    const label = closeLabel(rel.up, rel.down, g, t)
    if (rel.step) return withPrefix(t(`${K}.prefix.step`, {}), label)
    if (rel.half) return withPrefix(t(`${K}.prefix.half`, {}), label)
    return label
}

function partnerLabel(p: Partnership, g: GenderKey, t: Translate): string {
    const base = term(t, p.kind === 'marriage' ? 'spouse' : 'partner', g)
    // Ended = an end date OR an end reason (the API accepts a reason without a date).
    const open = (p.ended_on ?? null) === null && (p.end_reason ?? null) === null
    if (open) return base
    // The row records THAT it ended by death, not who died: one neutral suffix.
    if (p.end_reason === 'death') return base + t(`${K}.suffix.widowed`, {})
    return t(`${K}.prefix.ex`, {}) + base
}

function inLawLabel(rel: KinshipRelation, g: GenderKey, t: Translate): string {
    // The dedicated in-law words only fit plain relations; a step relation
    // across a partnership keeps its step label ("partner's step-father").
    if (!rel.step) {
        if (rel.up === 1 && rel.down === 0) return term(t, 'parentInLaw', g)
        if (rel.up === 0 && rel.down === 1) return term(t, 'childInLaw', g)
        if (rel.up === 1 && rel.down === 1) return term(t, 'siblingInLaw', g)
    }
    const relation = relativeLabel(rel, g, t)
    // A relative OF the partner ("partner's aunt") vs. the partner of a relative ("aunt by marriage").
    if (rel.in_law === 'via_own_partner') return t(`${K}.partnersRelative`, { relation })
    return relation + t(`${K}.suffix.byMarriage`, {})
}

export function kinshipLabel(rel: KinshipRelation, gender: string, t: Translate): string {
    const g = genderKey(gender)
    const partnership = rel.partnership ?? null
    if (partnership !== null) return partnerLabel(partnership, g, t)
    if ((rel.in_law ?? null) !== null) return inLawLabel(rel, g, t)
    return relativeLabel(rel, g, t)
}
