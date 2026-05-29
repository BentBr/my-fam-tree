const MAILPIT = process.env['MAILPIT_URL'] ?? 'http://mail.my-fam-tree.docker'

export interface MailpitMessage {
    subject: string
    text: string
}

export async function clearMailpit(): Promise<void> {
    await fetch(`${MAILPIT}/api/v1/messages`, { method: 'DELETE' })
}

export async function waitForEmail(matcher: (subject: string) => boolean, timeoutMs = 30_000): Promise<MailpitMessage> {
    const start = Date.now()
    while (Date.now() - start < timeoutMs) {
        const r = await fetch(`${MAILPIT}/api/v1/messages?limit=10`)
        const j = (await r.json()) as { messages?: Array<{ ID: string; Subject: string }> }
        for (const m of j.messages ?? []) {
            if (matcher(m.Subject)) {
                const d = await fetch(`${MAILPIT}/api/v1/message/${m.ID}`)
                const detail = (await d.json()) as { Subject: string; Text: string }
                return { subject: detail.Subject, text: detail.Text }
            }
        }
        await new Promise((resolve) => setTimeout(resolve, 500))
    }
    throw new Error('no matching email arrived in time')
}
