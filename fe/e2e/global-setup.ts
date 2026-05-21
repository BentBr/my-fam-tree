import { Socket } from 'node:net'

// Drain the Redis instance shared by the api so each E2E run starts from a
// clean rate-limiter state. Without this, repeated runs against the same
// emails (`gate-test@example.com`, `profile@example.com`, etc.) eventually
// trip `MAGIC_LINK_RATE_PER_EMAIL_PER_HOUR=5` and the magic-link send returns
// 429 — the page never reaches its "Check your inbox" state and every signIn
// helper fails. Mailpit is cleared per-test by `clearMailpit()`.

interface RedisEndpoint {
    host: string
    port: number
}

/**
 * Resolve the Redis (host, port) tuple from `REDIS_URL` if set, otherwise
 * the compose alias. The compose-network host (`redis.my-family.docker`)
 * resolves inside the Playwright container but not on a bare CI runner —
 * GitHub Actions exposes the redis service container at `localhost:6379`
 * via `REDIS_URL=redis://localhost:6379/0`. We only need the authority,
 * not the database number.
 */
function resolveRedisEndpoint(): RedisEndpoint {
    const raw = process.env['REDIS_URL']
    if (raw !== undefined && raw !== '') {
        try {
            const url = new URL(raw)
            const port = url.port !== '' ? Number.parseInt(url.port, 10) : 6379
            return { host: url.hostname, port }
        } catch {
            // Fall through to the compose default; the connect attempt will
            // surface the real issue if the URL is malformed.
        }
    }
    return { host: 'redis.my-family.docker', port: 6379 }
}

function flushRedis(): Promise<void> {
    return new Promise((resolve, reject) => {
        const endpoint = resolveRedisEndpoint()
        const socket = new Socket()
        const timer = setTimeout(() => {
            socket.destroy()
            reject(new Error(`redis flush timed out (${endpoint.host}:${endpoint.port})`))
        }, 5_000)
        socket.connect(endpoint.port, endpoint.host, () => {
            // Minimal RESP-2 frame: FLUSHDB has no args.
            socket.write('*1\r\n$7\r\nFLUSHDB\r\n')
        })
        socket.on('data', (data) => {
            clearTimeout(timer)
            socket.end()
            const reply = data.toString()
            if (reply.startsWith('+OK')) {
                resolve()
            } else {
                reject(new Error(`unexpected redis reply: ${reply.trim()}`))
            }
        })
        socket.on('error', (err) => {
            clearTimeout(timer)
            reject(err)
        })
    })
}

export default async function globalSetup(): Promise<void> {
    if (process.env['CI'] === 'true' || process.env['E2E_FLUSH_REDIS'] === 'true') {
        await flushRedis()
        console.log('[E2E Setup] flushed redis (rate-limit buckets cleared)')
    } else {
        console.log('[E2E Setup] skipping redis flush (set CI=true or E2E_FLUSH_REDIS=true to enable)')
    }
}
