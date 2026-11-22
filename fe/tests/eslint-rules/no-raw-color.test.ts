// @vitest-environment node
import { RuleTester } from 'eslint'
import { describe, it } from 'vitest'

import noRawColor from '../../eslint-rules/no-raw-color.js'

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
