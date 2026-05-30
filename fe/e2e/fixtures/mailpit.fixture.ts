const MAILPIT = process.env['MAILPIT_URL'] ?? 'http://mail.my-fam-tree.docker'

export interface MailpitMessage {
    subject: string
    text: string
}

interface MailpitListItem {
    ID: string
    Subject: string
    To?: Array<{ Address: string }>
}

export async function clearMailpit(): Promise<void> {
    await fetch(`${MAILPIT}/api/v1/messages`, { method: 'DELETE' })
}

/**
 * Wait for the next inbound email whose subject matches `matcher`.
 *
 * Pass `recipient` when the test owns the target address (which it almost
 * always does — sign-in / invite / email-change / owner-transfer all key
 * on a known mailbox). Without it, the matcher would also pick up a stale
 * email that landed in mailpit AFTER `clearMailpit` cleared the box —
 * e.g., a previous test's outbox row that the worker shipped late, or any
 * sibling test using the same subject template. Mailpit returns messages
 * newest-first, so a late stale email outranks the one we wanted, and the
 * consumer (ConsumeView / InviteAccept / EmailChangeConsumeView) gets the
 * stale token. The BE has already consumed it → `MagicLinkInvalid` → the
 * test fails with "the link may have expired". Filtering by `To` closes
 * that off, since each test uses unique stamped addresses.
 */
export async function waitForEmail(
    matcher: (subject: string) => boolean,
    options: { recipient?: string; timeoutMs?: number } = {},
): Promise<MailpitMessage> {
    const { recipient, timeoutMs = 30_000 } = options
    const wanted = recipient?.toLowerCase()
    const start = Date.now()
    while (Date.now() - start < timeoutMs) {
        const r = await fetch(`${MAILPIT}/api/v1/messages?limit=20`)
        const j = (await r.json()) as { messages?: MailpitListItem[] }
        for (const m of j.messages ?? []) {
            if (!matcher(m.Subject)) continue
            if (wanted !== undefined) {
                const to = m.To ?? []
                if (!to.some((t) => t.Address.toLowerCase() === wanted)) continue
            }
            const d = await fetch(`${MAILPIT}/api/v1/message/${m.ID}`)
            const detail = (await d.json()) as { Subject: string; Text: string }
            return { subject: detail.Subject, text: detail.Text }
        }
        await new Promise((resolve) => setTimeout(resolve, 500))
    }
    const tail = recipient !== undefined ? ` for ${recipient}` : ''
    throw new Error(`no matching email arrived${tail} in ${timeoutMs}ms`)
}
