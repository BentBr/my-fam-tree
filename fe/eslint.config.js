import vueI18n from '@intlify/eslint-plugin-vue-i18n'
import prettier from '@vue/eslint-config-prettier'
import { defineConfigWithVueTs, vueTsConfigs } from '@vue/eslint-config-typescript'
import vue from 'eslint-plugin-vue'
import globals from 'globals'

import noRawColor from './eslint-rules/no-raw-color.js'

const schemaImports = {
    group: ['@/api/schema*'],
    message: 'Import generated schema types only inside src/api/, not from views.',
}

/** Matches `@/<layer>/…` and relative `../<layer>/…` specifiers. `\x2F` is `/` (esquery regexes cannot hold one). */
function layerImportRegex(layers) {
    const alt = layers.join('|')
    return `^@\\x2F(${alt})\\x2F|(^|\\x2F)\\.\\.(\\x2F\\.\\.)*\\x2F(${alt})\\x2F`
}

/** Forbid static and dynamic imports from `files` into the `forbidden` src/ folders. */
function layer(files, forbidden) {
    const regex = layerImportRegex(forbidden)
    const message = `${files.join(', ')} must not import from src/{${forbidden.join(',')}}.`
    return {
        files,
        rules: {
            'no-restricted-imports': ['error', { patterns: [schemaImports, { regex, message }] }],
            'no-restricted-syntax': ['error', { selector: `ImportExpression[source.value=/${regex}/]`, message }],
        },
    }
}

export default defineConfigWithVueTs(
    { ignores: ['dist/**', 'src/api/schema.d.ts', 'playwright-report/**', 'test-results/**', 'coverage/**'] },
    vue.configs['flat/recommended'],
    vueTsConfigs.strictTypeChecked,
    prettier,
    {
        languageOptions: {
            ecmaVersion: 2022,
            sourceType: 'module',
            globals: { ...globals.browser, ...globals.node },
        },
        rules: {
            // Measured 187 hits, all numbers in template literals; numbers stringify safely.
            '@typescript-eslint/restrict-template-expressions': ['error', { allowNumber: true }],
            // Measured 44 hits (> 40): not adopted yet, pending Bent's decision.
            '@typescript-eslint/no-unnecessary-type-assertion': 'off',
            '@typescript-eslint/no-explicit-any': 'error',
            '@typescript-eslint/no-non-null-assertion': 'error',
            '@typescript-eslint/consistent-type-imports': 'error',
            '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
            'vue/multi-word-component-names': 'off',
            'vue/block-lang': ['error', { script: { lang: 'ts' } }],
            'vue/component-api-style': ['error', ['script-setup']],
            'vue/define-macros-order': 'error',
            'vue/no-v-html': 'error',
            'vue/no-unused-refs': 'error',
            'vue/no-useless-v-bind': 'error',
            'vue/require-typed-ref': 'error',
            'vue/no-undef-components': [
                'error',
                { ignorePatterns: ['^v-', '^router-(link|view)$', '^RouterLink$', '^RouterView$', '^i18n-t$'] },
            ],
            'no-restricted-imports': ['error', { patterns: [schemaImports] }],
        },
    },
    // Folder layers (src/router/** and src/views/** are unrestricted).
    layer(['src/components/**'], ['views']),
    layer(['src/stores/**', 'src/api/**', 'src/composables/**'], ['views', 'components']),
    layer(['src/utils/**', 'src/design-system/**', 'src/types/**'], ['views', 'components', 'stores', 'api']),
    {
        files: ['src/views/**/*.vue', 'src/components/**/*.vue'],
        plugins: { '@intlify/vue-i18n': vueI18n },
        rules: {
            // Punctuation-only text and technology names are not translatable copy.
            '@intlify/vue-i18n/no-raw-text': [
                'error',
                { ignorePattern: '^[\\s()·—:.❤-]+$', ignoreText: ['Rust', 'Vue.js'] },
            ],
        },
    },
    {
        files: ['src/**/*.{ts,vue}'],
        ignores: ['src/design-system/**'],
        plugins: { local: { rules: { 'no-raw-color': noRawColor } } },
        rules: { 'local/no-raw-color': 'error' },
    },
)
