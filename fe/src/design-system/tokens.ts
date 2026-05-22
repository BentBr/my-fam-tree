export const colorTokens = {
    light: {
        background: '#fafafa',
        surface: '#ffffff',
        primary: '#2563eb',
        'primary-darken-1': '#1d4ed8',
        secondary: '#db2777',
        success: '#16a34a',
        warning: '#d97706',
        error: '#dc2626',
        info: '#0ea5e9',
        'on-background': '#1f2937',
        'on-surface': '#1f2937',
        'on-primary': '#ffffff',
    },
    dark: {
        background: '#0b1220',
        surface: '#111827',
        primary: '#60a5fa',
        'primary-darken-1': '#3b82f6',
        secondary: '#f472b6',
        success: '#34d399',
        warning: '#fbbf24',
        error: '#f87171',
        info: '#38bdf8',
        'on-background': '#e5e7eb',
        'on-surface': '#e5e7eb',
        'on-primary': '#0b1220',
    },
} as const

export const radii = { sm: 4, md: 8, lg: 12, xl: 16, pill: 999 } as const
