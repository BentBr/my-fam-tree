import type { components } from './schema'

/**
 * Session claims returned by `/auth/consume`, `/auth/me`, and `/auth/refresh`.
 * Re-exported from `src/api/` so consumers (stores, hooks) can stay decoupled
 * from the generated schema barrel — see the `no-restricted-imports` rule.
 */
export type ClaimsPayload = components['schemas']['ConsumeRes']

/** A person row as returned by the persons endpoints (list + get). */
export type PersonView = components['schemas']['PersonView']
