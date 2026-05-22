import { describe, expect, it } from 'vitest'

import * as ds from '@/design-system'

// The barrel re-exports theme + tokens; verify both surface their primary
// exports so a future name change breaks the test, not the consumer.
describe('design-system barrel', () => {
    it('re-exports vuetifyTheme and colorTokens', () => {
        expect(ds.vuetifyTheme).toBeDefined()
        expect(ds.vuetifyDefaults).toBeDefined()
        expect(ds.colorTokens).toBeDefined()
        expect(ds.radii).toBeDefined()
    })
})
