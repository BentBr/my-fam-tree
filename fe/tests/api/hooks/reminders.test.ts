import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), PUT: vi.fn() },
}))

import { client } from '@/api/client'
import { type ReminderPrefs, useReminderPrefs, useSaveReminderPrefs } from '@/api/hooks/reminders'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
    PUT: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

const prefs: ReminderPrefs = {
    emails_enabled: true,
    remind_birthdays: true,
    remind_anniversaries: false,
    favourites_only: false,
    lead_days: 7,
}

beforeEach(() => {
    mocked.GET.mockReset()
    mocked.PUT.mockReset()
})

describe('useReminderPrefs', () => {
    it('GETs /reminder-preferences and unwraps the envelope', async () => {
        mocked.GET.mockResolvedValueOnce({ data: { data: prefs }, error: undefined })
        const { result } = makeHookWrapper(() => useReminderPrefs())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/reminder-preferences')
        expect(result.data.value).toEqual(prefs)
    })

    it('surfaces an error response', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useReminderPrefs())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value).toBeDefined()
    })
})

describe('useSaveReminderPrefs', () => {
    it('PUTs the prefs body', async () => {
        mocked.PUT.mockResolvedValueOnce({ data: { data: prefs }, error: undefined })
        const { result } = makeHookWrapper(() => useSaveReminderPrefs())
        await result.mutateAsync(prefs)
        expect(mocked.PUT).toHaveBeenCalledWith('/api/v1/reminder-preferences', { body: prefs })
    })

    it('rejects on error', async () => {
        mocked.PUT.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useSaveReminderPrefs())
        await expect(result.mutateAsync(prefs)).rejects.toBeDefined()
    })
})
