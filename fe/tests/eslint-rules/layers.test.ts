// @vitest-environment node
import { beforeAll, describe, expect, it } from 'vitest'

import { realConfigLinter } from './real-config'

const { warmUp, ruleHits: layerErrors } = realConfigLinter(['no-restricted-imports', 'no-restricted-syntax'])

describe('folder layer imports', () => {
    beforeAll(async () => {
        await warmUp('src/stores/x.ts')
    }, 60_000)

    it.each([
        ['src/stores/x.ts', "import A from '@/views/A.vue'\nexport { A }\n"],
        ['src/stores/x.ts', "import A from '../views/A.vue'\nexport { A }\n"],
        ['src/components/x.ts', "export const load = () => import('@/views/A.vue')\n"],
        ['src/api/hooks/x.ts', "export const load = () => import('../../components/B.vue')\n"],
        ['src/utils/x.ts', "import { useAuthStore } from '@/stores/auth'\nexport { useAuthStore }\n"],
        ['src/design-system/x.ts', "import { client } from '@/api/client'\nexport { client }\n"],
        ['src/types/x.ts', "import B from '../components/common/B.vue'\nexport { B }\n"],
    ])('rejects %s → %s', async (file, code) => {
        expect(await layerErrors(file, code)).toHaveLength(1)
    })

    it.each([
        ['src/api/hooks/x.ts', "import { client } from '../client'\nexport { client }\n"],
        [
            'src/views/A.vue',
            '<script setup lang="ts">\nimport { useAuthStore } from \'@/stores/auth\'\nuseAuthStore()\n</script>\n',
        ],
        ['src/router/index.ts', "export const load = () => import('@/views/A.vue')\n"],
        ['src/stores/x.ts', "import { client } from '@/api/client'\nexport { client }\n"],
        ['src/components/x.ts', "import { useUiStore } from '@/stores/ui'\nexport { useUiStore }\n"],
    ])('allows %s → %s', async (file, code) => {
        expect(await layerErrors(file, code)).toEqual([])
    })

    it('keeps the generated-schema restriction inside layered folders', async () => {
        const code = "import type { components } from '@/api/schema'\nexport type C = components\n"
        expect(await layerErrors('src/stores/x.ts', code)).toEqual(['no-restricted-imports'])
    })
})
