import { fileURLToPath, URL } from 'node:url'

import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'
import vuetify from 'vite-plugin-vuetify'

export default defineConfig({
    plugins: [vue(), vuetify({ autoImport: true })],
    resolve: {
        alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
    },
    server: {
        port: 5173,
        host: '0.0.0.0',
        strictPort: true,
        // Vite blocks requests whose Host header doesn't match the dev server.
        // Dinghy proxies `*.my-family.docker` to this container, so we have to
        // allow those hostnames explicitly. `localhost` covers host-side runs.
        allowedHosts: ['my-family.docker', 'app.my-family.docker', 'localhost'],
        proxy: {
            // Inside the `fe` compose container the API is reachable via the FQDN
            // alias `api.my-family.docker`. On a host-side `pnpm dev` it defaults
            // to localhost:8080. Set VITE_API_PROXY_TARGET to override.
            '/api': {
                target: process.env.VITE_API_PROXY_TARGET ?? 'http://localhost:8080',
                changeOrigin: true,
            },
        },
    },
})
