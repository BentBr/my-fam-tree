import { radii, spacing, transitions } from './tokens'

export const treeNodeStyle = {
    width: 200,
    height: 72,
    radius: radii.md,
    shadow: '0 2px 6px rgba(0,0,0,0.08)',
    shadowHover: '0 8px 24px rgba(37, 99, 235, 0.18)',
    ringColor: 'rgb(var(--v-theme-primary))',
    ringWidth: 2,
    fadeDuration: transitions.hover,
    padding: spacing.sm,
} as const
