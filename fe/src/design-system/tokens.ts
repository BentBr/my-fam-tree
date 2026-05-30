/**
 * Typed constants that live alongside `tokens.css`.
 *
 * All chrome / surface / text / accent colours are exclusively driven
 * by CSS custom properties (see `tokens.css`). The two tables below
 * exist *only* because they're consumed by code paths that need
 * literal RGB values at runtime — the SVG tree canvas (where edges
 * paint outside of Vue's reactive style binding) and the typed avatar
 * tone resolver. Both mirror values declared in `tokens.css`; **any
 * edit here must also land in `tokens.css`**.
 *
 * No other module should hard-code colours; always reach for
 * `var(--token)` in styles, or read from `tokens.css` via
 * `getComputedStyle(document.documentElement).getPropertyValue('--…')`.
 */

/** Avatar fills — one tone per sex, plus a grey override for the deceased. */
export const AVATAR_TONES = {
    female: { bg: '#E07A3A', fg: '#FFFFFF' },
    male: { bg: '#8A5A2E', fg: '#FFFFFF' },
    diverse: { bg: '#C9A266', fg: '#3A2A14' },
    deceased: { bg: '#9A938A', fg: '#FFFFFF' },
} as const

export type Sex = 'female' | 'male' | 'diverse' | 'unknown'

/** Resolve the avatar tone for a person. Falls back to `diverse` when the
 *  domain `sex` is `unknown` (we never want a bare grey on the living). */
export function avatarTone(sex: Sex, deceased: boolean): { bg: string; fg: string } {
    if (deceased) return AVATAR_TONES.deceased
    return AVATAR_TONES[sex === 'unknown' ? 'diverse' : sex]
}

/** Tree connector colours, per theme. Used by the SVG renderer where
 *  CSS variables can't reach (raw `<line stroke>` attributes). The Vue
 *  side of the tree resolves these through `tokens.css` instead. */
export const TREE_TOKENS = {
    light: { edge: '#C9BBAA', partner: '#D9568B', grid: '#EFE7DD' },
    dark: { edge: '#4A3E32', partner: '#E27CA6', grid: '#1B1611' },
} as const

/** Radii in pixels — match the `--r-*` custom properties. Used by the
 *  Vuetify `defaults.global.rounded` etc., not by hand-written CSS. */
export const radii = {
    xs: 6,
    sm: 9,
    md: 14,
    lg: 20,
    xl: 28,
    pill: 999,
} as const
