import type { components } from './schema'

/**
 * Session claims returned by `/auth/consume`, `/auth/me`, and `/auth/refresh`.
 * Re-exported from `src/api/` so consumers (stores, hooks) can stay decoupled
 * from the generated schema barrel — see the `no-restricted-imports` rule.
 */
export type ClaimsPayload = components['schemas']['ConsumeRes']

export type FamilyClaim = components['schemas']['FamilyClaim']

export type ApiRole = components['schemas']['Role']
