import { vi } from 'vitest'

import type { ClientSessionHooks } from '@/api/client'
import type { FamilyId } from '@/types/brand'

/** Spy-backed session hooks for `configureClient`; override any hook per test. */
export function fakeSessionHooks(overrides: Partial<ClientSessionHooks> = {}) {
    return {
        familyId: vi.fn((): FamilyId | null => null),
        refreshSession: vi.fn(() => Promise.resolve()),
        endSession: vi.fn(),
        notifyWarning: vi.fn(),
        ...overrides,
    }
}
