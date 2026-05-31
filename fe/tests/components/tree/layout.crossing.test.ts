// Pinned layout edge cases from a real family tree (Brüggemann tree,
// screenshot dated 2026-05-31). All three cases live as `it.todo(...)`
// — they're pending, not running, so the suite stays green. They exist
// to keep the cases visible until the layout pipeline handles them, at
// which point the test bodies get fleshed out against the same fixtures
// the seeder already inserts (see `crates/seeder/src/relationships.rs`
// — the "Krause" mini-subtree was added for exactly this purpose).
//
// The two global layout rules these encode:
//   1. Siblings sort by birthdate, oldest LEFT → youngest RIGHT, and
//      adding a spouse to one sibling must NOT shuffle the sibling row.
//   2. Same-row parent persons / in-married couples should be ordered
//      to AVOID crossing parent / partner edges with the row below
//      whenever a non-crossing order exists.
//
// Resume layout work via the upcoming-tree-layout-rules memory.

import { describe, it } from 'vitest'

describe('layoutTree — pinned regressions from the real Brüggemann tree', () => {
    it.todo(
        'sibling birth-date order is stable when one sibling gains a spouse ' +
            '(Krause family: Lars 1985 < Marie 1987 < Tim 1989 — Tim+Mia couple must not push Tim left of Marie)',
    )

    it.todo(
        'two unpartnered same-row mothers are ordered to match their children below ' +
            '(Krause family: Anneliese 1921 LEFT of Greta 1912 because Anneliese’s son Bernhard sits LEFT of Greta’s son Hubert)',
    )

    it.todo(
        'in-married couple sides are chosen to minimise parent-edge crossings ' +
            '(Krause family: Tim LEFT (his parents on the left), Mia RIGHT (her parents on the right) — opposite order crosses both edges)',
    )
})
