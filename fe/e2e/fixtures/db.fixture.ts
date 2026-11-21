import { Client } from 'pg'

// Direct DB access for state the API deliberately doesn't expose (a user's
// stored IANA timezone) and for asserting worker side effects that are
// written synchronously (reminder_digests rows created by a tick).
// Same resolution order as global-teardown.ts.
const DATABASE_URL =
    process.env['E2E_DATABASE_URL'] ??
    process.env['DATABASE_URL'] ??
    'postgres://my_fam_tree:my_fam_tree@postgres.my-fam-tree.docker:5432/my_fam_tree'

async function withDb<T>(fn: (client: Client) => Promise<T>): Promise<T> {
    const client = new Client({ connectionString: DATABASE_URL })
    await client.connect()
    try {
        return await fn(client)
    } finally {
        await client.end()
    }
}

/** Overwrite the stored timezone of the user with `email`. */
export async function setUserTimezone(email: string, timezone: string): Promise<void> {
    await withDb(async (c) => {
        const res = await c.query('UPDATE users SET timezone = $1 WHERE email = $2', [timezone, email])
        if (res.rowCount !== 1) throw new Error(`no user with email ${email}`)
    })
}

/** Send dates (`YYYY-MM-DD`) of every reminder digest scheduled for `email`. */
export async function digestSendDates(email: string): Promise<string[]> {
    return withDb(async (c) => {
        const res = await c.query<{ send_date: string }>(
            `SELECT d.send_date::text AS send_date
               FROM reminder_digests d JOIN users u ON u.id = d.user_id
              WHERE u.email = $1
              ORDER BY d.send_date`,
            [email],
        )
        return res.rows.map((r) => r.send_date)
    })
}
