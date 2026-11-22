// @vitest-environment node
import { RuleTester } from 'eslint'
import vue from 'eslint-plugin-vue'
import { beforeAll, describe, expect, it } from 'vitest'

import noRawColor from '../../eslint-rules/no-raw-color.js'

import { realConfigLinter } from './real-config'

RuleTester.describe = describe
RuleTester.it = it
RuleTester.itOnly = it.only

const ruleTester = new RuleTester({ languageOptions: { ecmaVersion: 2022, sourceType: 'module' } })
const error = { messageId: 'rawColor' }

ruleTester.run('no-raw-color', noRawColor, {
    valid: [
        "const href = '#/route-hash'",
        'const c = tokens.color.primary',
        "const border = '1px solid'",
        "// '#fff' in a comment\nconst x = 1",
        "const notHex = '#12345'",
        "const id = '#section'",
        'const tpl = `#/route/${id}`',
        "const themed = 'rgb(var(--v-theme-primary))'",
    ],
    invalid: [
        { code: "const c = '#fff'", errors: [error] },
        { code: "const c = '#A1B2C3'", errors: [error] },
        { code: "const c = '#a1b2c3d4'", errors: [error] },
        { code: "const c = 'rgb(1, 2, 3)'", errors: [error] },
        { code: "const c = 'RGBA(1, 2, 3, 0.5)'", errors: [error] },
        { code: "const c = 'hsl(0 0% 0%)'", errors: [error] },
        { code: 'const c = `#fff`', errors: [error] },
        { code: 'const c = `rgb(${r}, 0, 0)`', errors: [error] },
    ],
})

// eslint-plugin-vue's base config carries vue-eslint-parser (not a direct dependency).
const vueParser = vue.configs['flat/base'].find((c) => c.languageOptions?.['parser'] !== undefined)?.languageOptions?.[
    'parser'
]
if (vueParser === undefined) throw new Error('eslint-plugin-vue flat/base has no parser')
const vueTester = new RuleTester({ languageOptions: { parser: vueParser, ecmaVersion: 2022, sourceType: 'module' } })

/** A one-file SFC; `body` goes inside <template>, `style` into a <style> block. */
function sfc(body: string, style = ''): { code: string; filename: string } {
    const styleBlock = style === '' ? '' : `\n<style scoped>\n${style}\n</style>\n`
    return { code: `<template>\n    ${body}\n</template>\n${styleBlock}`, filename: 'src/components/X.vue' }
}

vueTester.run('no-raw-color (vue)', noRawColor, {
    valid: [
        sfc('<v-btn color="primary">x</v-btn>'),
        sfc('<div style="color: var(--text-strong)">x</div>'),
        sfc('<a href="#/route">x</a>'),
        sfc('<!-- #fff in a template comment --><div />'),
        sfc('<div :class="{ active: on }" />'),
        sfc(
            '<div class="x" />',
            '.x { color: rgba(var(--v-theme-on-surface), 0.5); background: rgb(var(--v-theme-primary) / 0.06); }',
        ),
        sfc('<div style="outline: 2px solid rgb(var(--v-theme-primary))" />'),
        sfc('<div class="x" />', '.x { color: var(--accent); border: 1px solid var(--line); }'),
        sfc('<div class="x" />', '/* was #fff, now a token */ .x { color: var(--accent); }'),
    ],
    invalid: [
        { ...sfc('<v-icon color="#fff" />'), errors: [{ ...error, line: 2 }] },
        { ...sfc('<div style="color: rgb(1, 2, 3)">x</div>'), errors: [error] },
        { ...sfc('<div style="border: 1px solid #A1B2C3; color: #fff">x</div>'), errors: [error, error] },
        { ...sfc(`<div :style="{ color: '#fff' }" />`), errors: [error] },
        {
            ...sfc('<div class="x" />', '.x {\n    color: #fff;\n    background: hsla(0, 0%, 0%, 0.5);\n}'),
            errors: [
                { ...error, line: 7, column: 12 },
                { ...error, line: 8, column: 17 },
            ],
        },
    ],
})

describe('no-raw-color in the real config', () => {
    const { warmUp, ruleHits } = realConfigLinter(['local/no-raw-color'])

    beforeAll(async () => {
        await warmUp('src/components/X.vue')
    }, 60_000)

    it('reports components and skips the design system', async () => {
        expect(await ruleHits('src/components/x.ts', "export const c = '#fff'\n")).toEqual(['local/no-raw-color'])
        expect(await ruleHits('src/design-system/x.ts', "export const c = '#fff'\n")).toEqual([])
    })

    it('reports colours in .vue templates and <style> blocks', async () => {
        const code =
            '<template>\n    <div class="x" style="color: #fff" />\n</template>\n<style scoped>\n.x { color: rgb(0, 0, 0); }\n</style>\n'
        expect(await ruleHits('src/components/X.vue', code)).toEqual(['local/no-raw-color', 'local/no-raw-color'])
    })
})
