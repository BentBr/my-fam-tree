import { Socket } from 'node:net'

// Drain the Redis instance shared by the api so the BE's per-IP rate
// limits don't carry over between tests. Without this, ~30-40 tests
// into the run, all requests come from 127.0.0.1 in CI and the
// `auth.rate_limit_ip` cap (120/hour, set at every magic-link consume
// + every /auth/refresh + every /invite/accept call site) fills up.
// Subsequent refresh attempts get 429s, which look identical to a
// genuine session-expiry bug — `refresh-after-access-cookie-gone`
// was the canary that surfaced this. We also clear at global setup
// (see `global-setup.ts`) so the *first* test starts from a clean
// slate; the per-test flush below keeps the same property all the
// way through the run.
//
// `FLUSHDB` rather than a targeted SCAN+DEL on `*rate:*`: the redis
// db hosts only the api's transient state (rate buckets, worker
// leader lease, reminder queue, idempotency keys). The lease
// auto-re-acquires within one tick, the queue is empty between
// tests, and idempotency is per-request — so a full flush is
// cheaper than a scan and doesn't introduce a per-test fragility on
// the worker side.

interface RedisEndpoint {
    host: string
    port: number
}

function resolveRedisEndpoint(): RedisEndpoint {
    const raw = process.env['REDIS_URL']
    if (raw !== undefined && raw !== '') {
        try {
            const url = new URL(raw)
            const port = url.port !== '' ? Number.parseInt(url.port, 10) : 6379
            return { host: url.hostname, port }
        } catch {
            // Fall through to the compose default.
        }
    }
    return { host: 'redis.my-fam-tree.docker', port: 6379 }
}

/**
 * Issue a single `FLUSHDB` to the Redis the api is using. Resolves
 * after the `+OK` reply. Rejects on connection failure or non-OK
 * reply. 5 s timeout — anything longer than that is a stack problem
 * the per-test guard shouldn't paper over.
 */
export function flushRedis(): Promise<void> {
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
