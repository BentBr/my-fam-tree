import prettier from '@vue/eslint-config-prettier'
import { defineConfigWithVueTs, vueTsConfigs } from '@vue/eslint-config-typescript'
import vue from 'eslint-plugin-vue'
import globals from 'globals'

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
            'no-restricted-imports': [
                'error',
                {
                    patterns: [
                        {
                            group: ['@/api/schema*'],
                            message: 'Import generated schema types only inside src/api/, not from views.',
                        },
                    ],
                },
            ],
        },
    },
)
