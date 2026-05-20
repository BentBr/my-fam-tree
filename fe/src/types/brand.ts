declare const __brand: unique symbol

export type Brand<T, B extends string> = T & { readonly [__brand]: B }

export type UserId = Brand<string, 'UserId'>
export type FamilyId = Brand<string, 'FamilyId'>
export type PersonId = Brand<string, 'PersonId'>
export type FamilyMembershipId = Brand<string, 'FamilyMembershipId'>

/**
 * Narrow a plain string (from the wire / generated types) to a branded id.
 * Use only inside `src/api/` adapters — never in view code.
 */
export const brand = {
    user: (s: string): UserId => s as UserId,
    family: (s: string): FamilyId => s as FamilyId,
    person: (s: string): PersonId => s as PersonId,
    familyMembership: (s: string): FamilyMembershipId => s as FamilyMembershipId,
} as const

/**
 * Escape hatch with audit trail. Logged in dev so reviewers can spot it.
 */
export function unsafeCast<T>(value: unknown, reason: string): T {
    if (import.meta.env.DEV) {
        console.warn(`[unsafeCast] ${reason}`)
    }
    return value as T
}
