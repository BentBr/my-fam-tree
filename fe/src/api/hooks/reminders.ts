import { useQuery } from '@tanstack/vue-query'

import { client } from '../client'
import { unwrap, useApiMutation } from '../request'

export interface ReminderPrefs {
    emails_enabled: boolean
    remind_birthdays: boolean
    remind_anniversaries: boolean
    favourites_only: boolean
    lead_days: number
}

/** GET the caller's reminder settings (server returns defaults if unsaved). */
export function useReminderPrefs() {
    return useQuery({
        queryKey: ['reminder-prefs'],
        queryFn: () => unwrap(client.GET('/api/v1/reminder-preferences')),
    })
}

/** PUT (upsert) the caller's reminder settings; toasts + refreshes the query. */
export function useSaveReminderPrefs() {
    return useApiMutation<ReminderPrefs, ReminderPrefs>({
        mutationFn: (prefs) => unwrap(client.PUT('/api/v1/reminder-preferences', { body: prefs })),
        success: 'toasts.reminder_prefs_saved',
        invalidate: () => [['reminder-prefs']],
    })
}
