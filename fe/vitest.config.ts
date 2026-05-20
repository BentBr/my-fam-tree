import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vitest/config'

export default defineConfig({
    resolve: {
        alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
    },
    test: {
        environment: 'happy-dom',
        include: ['tests/**/*.test.ts', 'src/**/*.test.ts'],
        // E2E tests live under e2e/ and are run by Playwright — exclude them
        // from Vitest's discovery so the two lanes don't collide.
        exclude: ['e2e/**', 'node_modules/**', 'dist/**', '.pnpm-store/**'],
    },
})
