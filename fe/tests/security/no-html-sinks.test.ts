// XSS structural invariant (Phase 5 Task 21 — security review).
//
// The FE renders every user-input field — person names, nickname, name-at-birth,
// gender, dates, place, notes, contact label/value, family names, audit
// metadata, … — through Vue's `{{ }}` interpolation, which HTML-escapes its
// output. The protection is structural: NO `v-html` / `innerHTML` / etc. sink
// exists anywhere in `fe/src`, so no field can reach the DOM as raw HTML.
//
// This test pins that invariant. If anyone introduces a raw-HTML sink later,
// it fails immediately — covering every existing AND every future field with
// one assertion, where per-field tests would only catch what's enumerated.

import { promises as fs } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const HERE = path.dirname(fileURLToPath(import.meta.url))
const SRC_DIR = path.resolve(HERE, '../../src')

// Patterns deliberately match actual *usage* (attribute, assignment, call), not
// the word in comments. Add to this list if a new sink shape appears upstream.
const SINK_PATTERNS: ReadonlyArray<readonly [string, RegExp]> = [
    ['v-html attribute', /\bv-html\s*=/],
    ['.innerHTML assignment', /\.innerHTML\s*=/],
    ['.outerHTML assignment', /\.outerHTML\s*=/],
    ['insertAdjacentHTML call', /\.insertAdjacentHTML\s*\(/],
    ['document.write call', /\bdocument\.write\s*\(/],
    ['eval call', /\beval\s*\(/],
    ['new Function call', /\bnew\s+Function\s*\(/],
    ['dangerouslySet (React-style)', /\bdangerouslySet/],
]

async function walkSources(dir: string): Promise<string[]> {
    const out: string[] = []
    const entries = await fs.readdir(dir, { withFileTypes: true })
    for (const e of entries) {
        const full = path.join(dir, e.name)
        if (e.isDirectory()) {
            out.push(...(await walkSources(full)))
        } else if (/\.(vue|ts|tsx|js)$/.test(e.name)) {
            out.push(full)
        }
    }
    return out
}

describe('XSS structural invariant', () => {
    it('fe/src contains zero raw-HTML / script sinks — every user input is auto-escaped', async () => {
        const files = await walkSources(SRC_DIR)
        expect(files.length, 'should find FE source files').toBeGreaterThan(0)

        const hits: string[] = []
        for (const file of files) {
            const content = await fs.readFile(file, 'utf8')
            for (const [label, re] of SINK_PATTERNS) {
                if (re.test(content)) {
                    hits.push(`${path.relative(process.cwd(), file)} → ${label}`)
                }
            }
        }

        expect(hits, hits.length === 0 ? '' : `Found sink(s):\n${hits.join('\n')}`).toEqual([])
    })
})
