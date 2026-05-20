// Runs once after the entire E2E suite (even when tests fail). Truncates any
// application-owned tables so the next CI run / local re-run starts from a
// clean slate. Built to be safe on a schema that has zero application tables
// (Phase 0d's migrations folder is empty) — the `information_schema` query
// returns an empty list and we exit early.

import { Client } from 'pg'

const DATABASE_URL =
    process.env['E2E_DATABASE_URL'] ?? 'postgres://my_family:my_family@postgres.my-family.docker:5432/my_family'

// Tables that belong to migrations infrastructure. Never truncate these.
const SYSTEM_TABLES = new Set(['_sqlx_migrations'])

export default async function globalTeardown(): Promise<void> {
    const client = new Client({ connectionString: DATABASE_URL })
    try {
        await client.connect()
        const res = await client.query<{ tablename: string }>(
            `SELECT tablename FROM pg_tables WHERE schemaname = 'public'`,
        )
        const targets = res.rows.map((r) => r.tablename).filter((name) => !SYSTEM_TABLES.has(name))
        if (targets.length === 0) {
            console.log('[E2E Teardown] no application tables to truncate')
            return
        }
        const quoted = targets.map((t) => `"${t}"`).join(', ')
        await client.query(`TRUNCATE TABLE ${quoted} RESTART IDENTITY CASCADE`)
        console.log(`[E2E Teardown] truncated ${targets.length} table(s): ${targets.join(', ')}`)
    } catch (error) {
        console.error('[E2E Teardown] cleanup failed (non-fatal):', error)
    } finally {
        await client.end().catch(() => undefined)
    }
}
