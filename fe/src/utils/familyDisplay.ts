// Family-list display helpers.
//
// Same-named families are allowed (no DB unique constraint), so the picker +
// switcher need a disambiguator. Per the design call, the date + role are
// surfaced ONLY when a name repeats — unique names stay clean. Helpers here
// are pure so they're trivially unit-testable and reusable across views.

export interface NamedFamily {
    readonly name: string
}

/** Names that appear more than once in `families`. */
export function duplicateNameSet<T extends NamedFamily>(families: readonly T[]): Set<string> {
    const counts = new Map<string, number>()
    for (const f of families) {
        counts.set(f.name, (counts.get(f.name) ?? 0) + 1)
    }
    const dups = new Set<string>()
    for (const [name, n] of counts) {
        if (n > 1) dups.add(name)
    }
    return dups
}

/**
 * Locale-aware human date for the family disambiguator (e.g. "1 May 2026").
 * Returns `null` when no timestamp is available — callers should fall back to
 * showing only the role in that case (data may still be loading).
 */
export function formatFamilyDate(iso: string | null | undefined, locale: 'en' | 'de'): string | null {
    if (iso === null || iso === undefined || iso === '') return null
    const d = new Date(iso)
    if (Number.isNaN(d.getTime())) return null
    return d.toLocaleDateString(locale === 'de' ? 'de-DE' : 'en-GB', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
    })
}
