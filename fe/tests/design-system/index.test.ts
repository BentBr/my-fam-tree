import { describe, expect, it } from 'vitest'

import * as ds from '@/design-system'

// The barrel re-exports theme + tokens; verify both surface their primary
// exports so a future name change breaks the test, not the consumer.
describe('design-system barrel', () => {
    it('re-exports the Vuetify theme + defaults', () => {
        expect(ds.vuetifyTheme).toBeDefined()
        expect(ds.vuetifyDefaults).toBeDefined()
    })

    it('re-exports the typed tree + avatar tone tables', () => {
        expect(ds.AVATAR_TONES).toBeDefined()
        expect(ds.TREE_TOKENS).toBeDefined()
        expect(ds.radii).toBeDefined()
    })
})
