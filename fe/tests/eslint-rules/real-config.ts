import { fileURLToPath } from 'node:url'

import { ESLint } from 'eslint'

const feRoot = fileURLToPath(new URL('../..', import.meta.url))

interface RealConfigLinter {
    /** Loads the flat config (vue + typescript-eslint); takes seconds on a cold container. */
    warmUp: (filePath: string) => Promise<void>
    /** Rule ids reported for `code` linted as if it lived at `filePath`. Throws on parse errors. */
    ruleHits: (filePath: string, code: string) => Promise<string[]>
}

/**
 * Lints in-memory snippets with the real `eslint.config.js`, limited to `ruleIds`.
 * Type-aware parsing is switched off because the snippet paths do not exist on disk.
 */
export function realConfigLinter(ruleIds: readonly string[]): RealConfigLinter {
    const wanted = new Set(ruleIds)
    const eslint = new ESLint({
        cwd: feRoot,
        ruleFilter: ({ ruleId }) => wanted.has(ruleId),
        overrideConfig: { languageOptions: { parserOptions: { projectService: false } } },
    })
    return {
        warmUp: async (filePath) => {
            await eslint.lintText('', { filePath })
        },
        ruleHits: async (filePath, code) => {
            const [result] = await eslint.lintText(code, { filePath })
            if (result === undefined) throw new Error(`no lint result for ${filePath}`)
            const fatal = result.messages.filter((m) => m.fatal === true)
            if (fatal.length > 0) {
                throw new Error(`parse error in ${filePath}: ${fatal.map((m) => m.message).join('; ')}`)
            }
            return result.messages.map((m) => m.ruleId ?? 'unknown')
        },
    }
}
