import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed } from 'vue'

import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'
import { unwrap } from '../request'

/**
 * `GET /api/v1/families/me` — the caller's families enriched with `created_at`
 * (the JWT family claim deliberately doesn't carry it, to keep the signed
 * token lean). Drives the picker's + switcher's "only-when-duplicated"
 * disambiguator. The switcher's family-change `invalidateQueries()` already
 * keys this back fresh on switches.
 */
export function useMyFamilies() {
    const auth = useAuthStore()
    // Gate on `authenticated`: FamilySwitcher (and therefore this hook) is
    // mounted inside AppBar, which lives in MainLayout. App.vue defaults the
    // layout to `'main'` while the route is still resolving on first load —
    // so AppBar can briefly mount before the auth guard's `auth.hydrate()`
    // settles. Firing this query while anonymous yields a 401 that the global
    // error handler surfaces as a misleading `session_expired` toast on top
    // of an otherwise-successful first navigation (e.g. /invite/accept).
    return useQuery({
        queryKey: ['families', 'me'] as const,
        enabled: computed(() => auth.status === 'authenticated'),
        queryFn: async () => {
            const payload = await unwrap(client.GET('/api/v1/families/me'))
            return payload.families
        },
    })
}

// Kept on raw `useMutation` (not `useApiMutation`): both hooks hydrate the
// auth store from the response and invalidate ALL queries (not a fixed key
// list), neither of which the wrapper models. The network call still routes
// through `unwrap()` so the error/empty handling is identical to every other
// hook. `unwrap` returns the inner `{ family, claims }` — call sites read
// `res.family.id`.
export function useCreateFamily() {
    const auth = useAuthStore()
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (name: string) => {
            const payload = await unwrap(client.POST('/api/v1/families', { body: { name } }))
            auth.applyClaimsPayload(payload.claims)
            return payload
        },
        onSuccess: () => {
            qc.invalidateQueries()
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.family_created') })
        },
    })
}

/**
 * `GET /api/v1/admin/family/overview` — aggregated stats for the
 * admin "Family" page. One round-trip returns the active family's
 * name, role, member count, person count, and the 3 most recently
 * added persons. Admin / owner only (BE enforces; the route guard
 * does too).
 */
export function useFamilyOverview() {
    const auth = useAuthStore()
    return useQuery({
        queryKey: ['admin', 'family', 'overview'] as const,
        enabled: computed(() => auth.status === 'authenticated'),
        queryFn: async () => {
            return await unwrap(client.GET('/api/v1/admin/family/overview'))
        },
    })
}

/**
 * `PATCH /api/v1/families/{id}` — rename. The path id is the active
 * family. Admin / owner only (BE enforces). On success we invalidate
 * the families list + the overview query and refresh the auth claims
 * so the switcher / header pick up the new name immediately.
 */
export function useRenameFamily() {
    const auth = useAuthStore()
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (input: { id: string; name: string }) => {
            return await unwrap(
                client.PATCH('/api/v1/families/{id}', {
                    params: { path: { id: input.id } },
                    body: { name: input.name },
                }),
            )
        },
        onSuccess: (_, vars) => {
            // Rebuild the auth-store family list manually so the switcher's
            // current selection picks up the new name before any background
            // refetch lands. The BE doesn't reissue the JWT on rename (the
            // name isn't in the access token), so this is the only update
            // path for the in-memory store.
            const families = auth.families.map((f) =>
                f.id === vars.id ? { ...f, name: vars.name } : f,
            )
            auth.applyClaimsPayload({
                user_id: auth.user?.id ?? '',
                email: auth.user?.email ?? '',
                locale: auth.user?.locale ?? 'en',
                families,
            } as never)
            void qc.invalidateQueries({ queryKey: ['families', 'me'] })
            void qc.invalidateQueries({ queryKey: ['admin', 'family', 'overview'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.family_renamed') })
        },
    })
}

export function useAcceptInvite() {
    const auth = useAuthStore()
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (token: string) => {
            const payload = await unwrap(client.POST('/api/v1/invites/accept', { body: { token } }))
            auth.applyClaimsPayload(payload.claims)
            return payload
        },
        onSuccess: () => {
            qc.invalidateQueries()
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.invite_accepted') })
        },
    })
}
