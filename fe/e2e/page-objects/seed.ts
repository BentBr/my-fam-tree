import { type APIRequestContext, expect } from '@playwright/test'

/**
 * API seeding primitives shared across the e2e suite. Every spec that
 * builds a family graph (tree layout, big-family stress, deep-link,
 * favourites, …) was hand-rolling the same person / parent-link /
 * partnership POST shapes against `/api/v1/*` with the `X-Family-Id`
 * header. These helpers are that shape in one place so a field rename
 * on the backend only touches this file.
 *
 * All take an `APIRequestContext` (usually `page.request`) so they reuse
 * the browser's cookie jar — no UI clicks, no per-call auth.
 */

/** Create a person and return its server-assigned UUID. `data` is the raw
 * `PersonCreateReq` body — callers pass whatever fields they need. */
export async function seedPerson(
    request: APIRequestContext,
    familyId: string,
    data: Record<string, unknown>,
): Promise<string> {
    const res = await request.post('/api/v1/persons', {
        headers: { 'X-Family-Id': familyId },
        data,
    })
    expect(res.ok()).toBeTruthy()
    const body = (await res.json()) as { data: { id: string } }
    return body.data.id
}

/** Link `childId` to `parentId`. Logs the response body on failure so a
 * 4xx in a seed step is debuggable instead of a bare assertion. */
export async function seedParentLink(
    request: APIRequestContext,
    familyId: string,
    childId: string,
    parentId: string,
    kind = 'biological',
): Promise<void> {
    const res = await request.post('/api/v1/parent-links', {
        headers: { 'X-Family-Id': familyId },
        data: { child_id: childId, parent_id: parentId, kind },
    })
    if (!res.ok()) {
        console.error(`parent-link POST failed ${String(res.status())}:`, await res.text())
    }
    expect(res.ok()).toBeTruthy()
}

/** Create a contact on `personId`. `data` is the raw `/contacts` body
 * (`{ kind, value, visibility, label? }`). */
export async function seedContact(
    request: APIRequestContext,
    familyId: string,
    personId: string,
    data: Record<string, unknown>,
): Promise<void> {
    const res = await request.post(`/api/v1/persons/${personId}/contacts`, {
        headers: { 'X-Family-Id': familyId },
        data,
    })
    expect(res.ok()).toBeTruthy()
}

/** Create a partnership between two persons. Field names match
 * `PartnershipCreateReq` (`partner_a_id` / `partner_b_id`). */
export async function seedPartnership(
    request: APIRequestContext,
    familyId: string,
    aId: string,
    bId: string,
    opts?: { ended_on?: string; end_reason?: string; kind?: string },
): Promise<void> {
    const data: Record<string, unknown> = {
        partner_a_id: aId,
        partner_b_id: bId,
        kind: opts?.kind ?? 'marriage',
    }
    if (opts?.ended_on !== undefined) data['ended_on'] = opts.ended_on
    if (opts?.end_reason !== undefined) data['end_reason'] = opts.end_reason
    const res = await request.post('/api/v1/partnerships', {
        headers: { 'X-Family-Id': familyId },
        data,
    })
    if (!res.ok()) {
        console.error(`partnership POST failed ${String(res.status())}:`, await res.text())
    }
    expect(res.ok()).toBeTruthy()
}

/**
 * Synthetic multi-generation family for the big-family stress test:
 * `GENERATIONS` × `PER_GEN` persons with one parent link per child to
 * the same column one generation up. Sized to sit under the persons-list
 * 100-row server-side clamp; a wider version is gated behind raising that
 * limit + wiring cursor pagination into the tree service. 50 persons is
 * still plenty to catch the overlap / drop-node regressions the test
 * guards against.
 */
export const GENERATIONS = 10
export const PER_GEN = 5
export const TOTAL = GENERATIONS * PER_GEN

/** Map of synthetic id (`g0-3`) → server-assigned uuid. */
export type BulkResult = { idMap: Map<string, string> }

/** Seed key of the i-th person in generation g. */
function gridKey(g: number, i: number): string {
    return `g${String(g)}-${String(i)}`
}

export async function bulkSeedFamily(request: APIRequestContext, familyId: string): Promise<BulkResult> {
    const idMap = new Map<string, string>()

    // Persons go first, chunked so we don't open more than 50 sockets at
    // once. Each chunk is awaited before the next launches; within a chunk
    // the 50 POSTs race in parallel.
    const CHUNK = 50
    const personRequests: Array<() => Promise<void>> = []
    for (let g = 0; g < GENERATIONS; g += 1) {
        for (let i = 0; i < PER_GEN; i += 1) {
            const key = gridKey(g, i)
            personRequests.push(async () => {
                const id = await seedPerson(request, familyId, {
                    given_name: `P${String(g)}_${String(i)}`,
                    family_name: 'Stress',
                    // Distribute birth years so the generation-rank algorithm
                    // has signal: gen 0 ~1900, gen 9 ~1980.
                    birth_date: `${String(1900 + g * 9)}-01-01`,
                })
                idMap.set(key, id)
            })
        }
    }
    for (let i = 0; i < personRequests.length; i += CHUNK) {
        await Promise.all(personRequests.slice(i, i + CHUNK).map((fn) => fn()))
    }

    // Parent links: each (g,i) for g >= 1 gets one parent at (g-1, i).
    // A second parent at (g-1, (i+1) % PER_GEN) lets the partner pass
    // and the duplicate detection both have something to chew on.
    //
    // Parent-link inserts run in a SERIALIZABLE Postgres tx; concurrent
    // POSTs hitting the same family's row set get aborted with sqlstate
    // 40001. Sequential is fast enough at this scale (~50ms × ~200 ≈ 10s)
    // and dodges the abort/retry dance.
    for (let g = 1; g < GENERATIONS; g += 1) {
        for (let i = 0; i < PER_GEN; i += 1) {
            const childId = idMap.get(gridKey(g, i))
            const parentId = idMap.get(gridKey(g - 1, i))
            if (childId === undefined || parentId === undefined) continue
            await seedParentLink(request, familyId, childId, parentId)
        }
    }

    return { idMap }
}
