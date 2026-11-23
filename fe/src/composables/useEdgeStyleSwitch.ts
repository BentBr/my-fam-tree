import { useUpdateMe } from '@/api/hooks/users'
import { useAuthStore } from '@/stores/auth'
import { useUiStore, type EdgeStyle } from '@/stores/ui'

/**
 * THE tree connector-style switch, shared by both pickers (the desktop
 * `EdgeStyleToggle` and the mobile fold-in inside `AccountControl`).
 * Deliberately the same shape as `useLocaleSwitch`:
 *
 *   1. flip the ui store (drives the canvas + persists to localStorage),
 *   2. write the preference through to `/users/me` for signed-in users.
 *
 * There is no URL step — unlike the locale, the style is not part of the
 * route.
 */
export function useEdgeStyleSwitch(): (next: EdgeStyle) => void {
    const ui = useUiStore()
    const auth = useAuthStore()
    const update = useUpdateMe()

    return (next: EdgeStyle): void => {
        if (next === ui.edgeStyle) return
        ui.setEdgeStyle(next)
        if (auth.status === 'authenticated') {
            // Optimistic, like the locale: the in-memory account mirrors the
            // choice at once; the PATCH persists it and `useUpdateMe.onSuccess`
            // re-applies the confirmed value (idempotent).
            auth.patchUser({ edgeStyle: next })
            update.mutate({ edge_style: next })
        }
    }
}
