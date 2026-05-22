import prettier from '@vue/eslint-config-prettier'
import vueTypescript from '@vue/eslint-config-typescript'
import vue from 'eslint-plugin-vue'
import globals from 'globals'

export default [
    { ignores: ['dist/**', 'src/api/schema.d.ts', 'playwright-report/**', 'test-results/**', 'coverage/**'] },
    ...vue.configs['flat/recommended'],
    ...vueTypescript(),
    prettier,
    {
        languageOptions: {
            ecmaVersion: 2022,
            sourceType: 'module',
            globals: { ...globals.browser, ...globals.node },
        },
        rules: {
            '@typescript-eslint/no-explicit-any': 'error',
            '@typescript-eslint/no-non-null-assertion': 'error',
            '@typescript-eslint/consistent-type-imports': 'error',
            '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
            'vue/multi-word-component-names': 'off',
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
]
