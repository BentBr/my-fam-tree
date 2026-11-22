import { computed, type ComputedRef, type MaybeRefOrGetter, toValue } from 'vue'

import { NODE_W, type Positioned } from './layout'

// Width budget for the text columns (right of the avatar circle). Used to
// truncate the full name with ellipsis when it would overflow the card.
export const TEXT_LEFT = 64
export const TEXT_RIGHT_PAD = 12
const TEXT_WIDTH = NODE_W - TEXT_LEFT - TEXT_RIGHT_PAD

// Average glyph width for `system-ui` at the sizes we use. Cheap heuristic
// — better than measuring per-glyph and good enough for ellipsizing names
// that overflow the card. The card is 220px wide; the budget here lands
// around 18-22 chars for the name and ~24 for the smaller dates row.
const NAME_AVG_CHAR_PX = 7.5
const DATES_AVG_CHAR_PX = 5.8
const DATES_MAX_CHARS = Math.floor(TEXT_WIDTH / DATES_AVG_CHAR_PX)

function truncate(s: string, maxChars: number): string {
    if (s.length <= maxChars) return s
    if (maxChars <= 1) return '…'
    return `${s.slice(0, maxChars - 1)}…`
}

/**
 * Parse a (possibly partial) ISO date string into a Date. Accepts the
 * full `YYYY-MM-DD` shape, `YYYY-MM`, and bare `YYYY`. Returns null on
 * anything else. We hand-parse rather than feeding partial ISO to the
 * `Date` constructor — Safari rejects bare `YYYY-MM` and Chromium
 * interprets it as UTC midnight which can land on the previous local
 * day depending on the runtime tz.
 */
function parseIsoDate(s: string): Date | null {
    const parts = s.match(/^(\d{4})(?:-(\d{2}))?(?:-(\d{2}))?/)
    if (parts === null) return null
    const head = parts[1]
    if (head === undefined) return null
    const yr = Number.parseInt(head, 10)
    if (!Number.isFinite(yr)) return null
    const mo = parts[2] !== undefined ? Number.parseInt(parts[2], 10) - 1 : 0
    const da = parts[3] !== undefined ? Number.parseInt(parts[3], 10) : 1
    return new Date(yr, mo, da)
}

/**
 * Full years between two dates, day-precision. Canonical "how old are
 * you" semantics — subtracts a year if the end month/day hasn't reached
 * the start month/day yet.
 */
function fullYearsBetween(from: Date, to: Date): number {
    let years = to.getFullYear() - from.getFullYear()
    if (to.getMonth() < from.getMonth() || (to.getMonth() === from.getMonth() && to.getDate() < from.getDate())) {
        years -= 1
    }
    return years
}

interface NodeLabels {
    fullName: ComputedRef<string>
    birthLabel: ComputedRef<string>
    deathLabel: ComputedRef<string>
    hasDates: ComputedRef<boolean>
    ageLabel: ComputedRef<string>
    initials: ComputedRef<string>
}

/** Display strings for a tree-node card (name, dates, age, initials). */
export function useNodeLabels(source: MaybeRefOrGetter<Positioned>): NodeLabels {
    const fullName = computed(() => {
        const node = toValue(source)
        const raw = `${node.given_name} ${node.family_name}`.trim()
        return truncate(raw, Math.floor(TEXT_WIDTH / NAME_AVG_CHAR_PX))
    })

    const birthLabel = computed(() => {
        const b = toValue(source).birth_date ?? ''
        if (b === '') return ''
        return truncate(`* ${b}`, DATES_MAX_CHARS)
    })

    const deathLabel = computed(() => {
        const d = toValue(source).death_date ?? ''
        if (d === '') return ''
        return truncate(`† ${d}`, DATES_MAX_CHARS)
    })

    const hasDates = computed(() => birthLabel.value !== '' || deathLabel.value !== '')

    /**
     * Age label shown right-aligned on the birth date row. Living: just the
     * number. Deceased: `N (†)` to mark age-at-death. Returns the empty
     * string when birth_date is missing or unparseable — no cell rendered.
     */
    const ageLabel = computed(() => {
        const node = toValue(source)
        const birth = node.birth_date ?? ''
        if (birth === '') return ''
        const birthDate = parseIsoDate(birth)
        if (birthDate === null) return ''
        const deathStr = node.death_date ?? ''
        if (deathStr === '') {
            const years = fullYearsBetween(birthDate, new Date())
            return years >= 0 ? String(years) : ''
        }
        const deathDate = parseIsoDate(deathStr)
        if (deathDate === null) return ''
        const years = fullYearsBetween(birthDate, deathDate)
        return years >= 0 ? `${String(years)} (†)` : ''
    })

    const initials = computed(() => {
        const node = toValue(source)
        const combined = `${node.given_name.charAt(0)}${node.family_name.charAt(0)}`.toUpperCase()
        return combined === '' ? '?' : combined
    })

    return { fullName, birthLabel, deathLabel, hasDates, ageLabel, initials }
}
