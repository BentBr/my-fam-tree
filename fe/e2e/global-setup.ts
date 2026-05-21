import { Socket } from 'node:net'

// Drain the Redis instance shared by the api so each E2E run starts from a
// clean rate-limiter state. Without this, repeated runs against the same
// emails (`gate-test@example.com`, `profile@example.com`, etc.) eventually
// trip `MAGIC_LINK_RATE_PER_EMAIL_PER_HOUR=5` and the magic-link send returns
// 429 — the page never reaches its "Check your inbox" state and every signIn
// helper fails. Mailpit is cleared per-test by `clearMailpit()`.
function flushRedis(): Promise<void> {
    return new Promise((resolve, reject) => {
        const socket = new Socket()
        const timer = setTimeout(() => {
            socket.destroy()
            reject(new Error('redis flush timed out'))
        }, 5_000)
        socket.connect(6379, 'redis.my-family.docker', () => {
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
