// @vitest-environment node
import { beforeAll, describe, expect, it } from 'vitest'

import { realConfigLinter } from './real-config'

const { warmUp, ruleHits } = realConfigLinter(['import-x/no-cycle'])

// The snippet is linted as if it replaced a real file; its imports resolve
// against the real modules on disk, which import that file back.
describe('import cycles', () => {
    beforeAll(async () => {
        await warmUp('src/api/errors.ts')
    }, 60_000)

    it('reports a cycle through a real module (api/errors -> api/client -> api/errors)', async () => {
        const code = "import { client } from './client'\nexport { client }\n"
        expect(await ruleHits('src/api/errors.ts', code)).toEqual(['import-x/no-cycle'])
    })

    it('reports a cycle through an @/ alias import', async () => {
        const code = "import { client } from '@/api/client'\nexport { client }\n"
        expect(await ruleHits('src/api/errors.ts', code)).toEqual(['import-x/no-cycle'])
    })

    it('reports a cycle that runs through a .vue file (useRouteMeta -> App.vue -> useRouteMeta)', async () => {
        const code = "import App from '@/App.vue'\nexport { App }\n"
        expect(await ruleHits('src/composables/useRouteMeta.ts', code)).toEqual(['import-x/no-cycle'])
    })

    it('allows the stores -> api direction', async () => {
        const code = "import { client } from '@/api/client'\nexport { client }\n"
        expect(await ruleHits('src/stores/auth.ts', code)).toEqual([])
    })
})
