<script setup lang="ts">
/**
 * Right-edge account control of the AppBar.
 *
 * One activator + one dropdown for every auth state. The activator
 * swaps its glyph based on what visual identity the user has:
 *
 *   - photo uploaded         → DefaultAvatar with the image
 *   - display name set       → DefaultAvatar with initials
 *   - otherwise (anonymous   → outlined `user` icon tinted with the
 *     OR pre-profile authed)   sloth-orange accent
 *
 * The dropdown content swaps with auth state:
 *
 *   logged out → Login (primary) + Register   (both → /auth/sign-in)
 *   logged in  → Account (→ /account) + Sign out
 *
 * On phones (`smAndDown`) the AppBar's `ThemeToggle` and `LanguageMenu`
 * fold into this menu so the right edge isn't overcrowded — see the
 * extra items below. Desktop keeps the inline toggles.
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useDisplay } from 'vuetify'

import { useMe } from '@/api/hooks/users'
import DefaultAvatar from '@/components/common/DefaultAvatar.vue'
import { currentResolvedTheme } from '@/composables/useThemeMode'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore, type SupportedLocale } from '@/stores/locale'
import { useUiStore } from '@/stores/ui'

const { t } = useI18n()
const auth = useAuthStore()
const router = useRouter()
const ui = useUiStore()
const localeStore = useLocaleStore()
const { smAndDown } = useDisplay()

const me = useMe()
const isAuthed = computed(() => auth.status === 'authenticated')

// Visual-identity gate for the activator. DefaultAvatar only takes over
// once the user has uploaded a photo OR set a display name explicitly
// (an email-derived initial isn't enough — pre-profile users still get
// the generic orange user icon, matching the anonymous look). Email is
// kept as the AccountControl's accessible name so screen readers and
// e2e selectors can still locate "the user button".
const avatarUrl = computed(() => (isAuthed.value ? (me.data.value?.avatar_url ?? null) : null))
const explicitDisplayName = computed(() => {
    if (!isAuthed.value) return ''
    return (me.data.value?.display_name ?? '').trim()
})
const hasVisualIdentity = computed(() => avatarUrl.value !== null || explicitDisplayName.value !== '')
const activatorLabel = computed(() => (isAuthed.value ? (auth.user?.email ?? '') : t('chrome.account.label')))

// Mobile fold-in: the AppBar's inline ThemeToggle + LanguageMenu go
// away on `smAndDown` and reappear as items inside this menu.
const showInlineToolsInMenu = computed(() => smAndDown.value)

const resolved = computed(() => currentResolvedTheme(ui.theme))
const themeIcon = computed(() => (resolved.value === 'dark' ? 'sun' : 'moon'))
const themeLabel = computed(() =>
    resolved.value === 'dark' ? t('chrome.theme.switchToLight') : t('chrome.theme.switchToDark'),
)
function toggleTheme(): void {
    ui.setTheme(resolved.value === 'dark' ? 'light' : 'dark')
}

interface LangChoice {
    value: SupportedLocale
    label: string
    flag: string
}
const langChoices: LangChoice[] = [
    { value: 'en', label: 'English', flag: '🇬🇧' },
    { value: 'de', label: 'Deutsch', flag: '🇩🇪' },
]
function pickLocale(next: SupportedLocale): void {
    if (next === localeStore.locale) return
    localeStore.set(next)
}

async function signOut(): Promise<void> {
    await auth.logout()
    await router.replace('/auth/sign-in')
}

// Login + Register both target the same magic-link page. Plain `to="..."`
// on both `<v-list-item>`s collides on Vuetify's list-nav internal id
// tracking ("Multiple nodes with the same ID /auth/sign-in"), so we
// drive the navigation via `@click` and `router.push` instead.
function goToSignIn(): void {
    void router.push('/auth/sign-in')
}
</script>

<template>
    <v-menu location="bottom end">
        <template #activator="{ props: activatorProps }">
            <v-btn
                icon
                :title="activatorLabel"
                :aria-label="activatorLabel"
                data-testid="user-menu"
                v-bind="activatorProps"
            >
                <!-- A genuine avatar (photo or initials) only shows when the
                     user has earned one. Until then the orange user icon
                     stands in everywhere — same activator for the public
                     home, the sign-in screen, and a fresh-signed-in account
                     without a profile. -->
                <span v-if="hasVisualIdentity" aria-hidden="true">
                    <DefaultAvatar :src="avatarUrl" :name="explicitDisplayName" :size="36" />
                </span>
                <v-icon v-else icon="user" size="22" color="primary" aria-hidden="true" />
            </v-btn>
        </template>
        <v-list density="comfortable" data-testid="account-menu">
            <!-- Mobile-only fold-in of the AppBar's inline tools. Theme
                 + language stay always-available per the chrome contract,
                 just relocated here when there's no room on the AppBar. -->
            <template v-if="showInlineToolsInMenu">
                <v-list-item
                    :prepend-icon="themeIcon"
                    :title="themeLabel"
                    data-testid="account-menu-theme"
                    @click="toggleTheme"
                />
                <v-list-subheader>{{ t('chrome.language.label') }}</v-list-subheader>
                <v-list-item
                    v-for="c in langChoices"
                    :key="c.value"
                    :title="c.label"
                    :active="localeStore.locale === c.value"
                    :data-testid="`account-menu-locale-${c.value}`"
                    color="primary"
                    @click="pickLocale(c.value)"
                >
                    <template #prepend>
                        <span class="lang-flag" aria-hidden="true">{{ c.flag }}</span>
                    </template>
                </v-list-item>
                <v-divider class="my-1" />
            </template>

            <template v-if="isAuthed">
                <v-list-item
                    to="/account"
                    prepend-icon="user"
                    :title="t('account.menu.openAccount')"
                    data-testid="user-menu-account"
                />
                <v-list-item
                    prepend-icon="log-out"
                    :title="t('account.menu.signOut')"
                    data-testid="sign-out"
                    @click="signOut"
                />
            </template>
            <template v-else>
                <v-list-item
                    prepend-icon="log-in"
                    :title="t('chrome.account.login')"
                    data-testid="account-login"
                    color="primary"
                    @click="goToSignIn"
                />
                <v-list-item
                    prepend-icon="user-plus"
                    :title="t('chrome.account.register')"
                    data-testid="account-register"
                    @click="goToSignIn"
                />
            </template>
        </v-list>
    </v-menu>
</template>

<style scoped>
.lang-flag {
    font-size: 18px;
    line-height: 1;
    display: inline-block;
    margin-inline-end: 14px;
}
</style>
