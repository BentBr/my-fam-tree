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
        coverage: {
            provider: 'v8',
            reporter: ['text', 'json-summary', 'lcov'],
            reportsDirectory: './coverage',
            include: ['src/**/*.{ts,vue}'],
            // Generated / framework-wiring files: schema is openapi-typescript
            // output; main.ts is the app bootstrap (untestable as a unit);
            // vite-env.d.ts and other ambient .d.ts files contain only types.
            // Tests themselves are excluded so they don't count toward coverage.
            exclude: [
                'src/api/schema.d.ts',
                'src/main.ts',
                'src/vite-env.d.ts',
                'src/**/*.d.ts',
                'src/**/*.test.ts',
            ],
            thresholds: {
                // Lines only, matching backend's `--fail-under-lines` gate.
                // Branch / statement / function coverage are reported but not
                // gated. See spec Section 9 (Out of scope).
                lines: 80,
            },
        },
    },
})
