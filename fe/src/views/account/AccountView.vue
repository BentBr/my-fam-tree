<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useClearMyAvatar, useMe, useRequestEmailChange, useSetMyAvatar, useUpdateMe } from '@/api/hooks/users'
import DefaultAvatar from '@/components/common/DefaultAvatar.vue'
import { useAuthStore } from '@/stores/auth'
import ReminderPrefsSection from '@/views/account/ReminderPrefsSection.vue'

const { t } = useI18n()
const router = useRouter()
const auth = useAuthStore()
const me = useMe()
const update = useUpdateMe()
const requestChange = useRequestEmailChange()
const setAvatar = useSetMyAvatar()
const clearAvatar = useClearMyAvatar()
const avatarInput = ref<HTMLInputElement | null>(null)
const avatarBusy = computed(() => setAvatar.isPending.value || clearAvatar.isPending.value)

function openAvatarPicker(): void {
    avatarInput.value?.click()
}

async function onAvatarSelected(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement
    const file = input.files?.[0] ?? null
    // Reset BEFORE the await so the same file can be re-selected.
    input.value = ''
    if (file === null) return
    await setAvatar.mutateAsync(file)
}

async function removeAvatar(): Promise<void> {
    await clearAvatar.mutateAsync(undefined)
}

const displayName = ref('')
const localeSelected = ref<'en' | 'de'>('en')
const newEmail = ref('')
const errorMsg = ref<string | null>(null)
const changeRequested = ref(false)

// Sync form fields when /users/me arrives. Without `immediate`, the form would
// stay empty until the first cache update post-mount.
watch(
    () => me.data.value,
    (profile) => {
        if (profile === undefined) return
        displayName.value = profile.display_name
        if (profile.locale === 'en' || profile.locale === 'de') {
            localeSelected.value = profile.locale
        }
    },
    { immediate: true },
)

const localeItems = computed(() => [
    { value: 'en', title: t('language.en') },
    { value: 'de', title: t('language.de') },
])

const currentEmail = computed(() => me.data.value?.email ?? '')

async function saveProfile(): Promise<void> {
    errorMsg.value = null
    try {
        await update.mutateAsync({
            display_name: displayName.value.trim(),
            locale: localeSelected.value,
        })
    } catch (e: unknown) {
        errorMsg.value = e instanceof Error ? e.message : 'unknown error'
    }
}

async function submitEmailChange(): Promise<void> {
    errorMsg.value = null
    try {
        await requestChange.mutateAsync(newEmail.value.trim().toLowerCase())
        changeRequested.value = true
        newEmail.value = ''
    } catch (e: unknown) {
        errorMsg.value = e instanceof Error ? e.message : 'unknown error'
    }
}

async function signOut(): Promise<void> {
    await auth.logout()
    await router.replace('/auth/sign-in')
}
</script>

<template>
    <v-container max-width="720">
        <v-card class="pa-6" data-testid="account-card">
            <v-card-title class="text-h5 mb-4">{{ t('account.title') }}</v-card-title>
            <v-alert v-if="errorMsg" type="error" class="mb-4" data-testid="account-error">
                {{ errorMsg }}
            </v-alert>

            <v-card-subtitle class="text-h6 px-0 mt-2">
                {{ t('account.profile.title') }}
            </v-card-subtitle>
            <!-- Avatar slot: click the avatar to upload, dedicated remove
                 button when one is already set. Identical UX shape to the
                 person photo upload in PersonDetail. -->
            <div class="d-flex align-center ga-3 mb-4">
                <div class="position-relative">
                    <DefaultAvatar
                        :src="me.data.value?.avatar_url ?? null"
                        :name="displayName || me.data.value?.email || ''"
                        :size="72"
                        data-testid="account-avatar"
                    />
                    <v-btn
                        icon="mdi-camera"
                        size="x-small"
                        color="primary"
                        class="account-avatar-edit"
                        :loading="avatarBusy"
                        :aria-label="t('account.avatar.upload')"
                        data-testid="account-avatar-upload"
                        @click="openAvatarPicker"
                    />
                    <input
                        ref="avatarInput"
                        type="file"
                        accept="image/jpeg,image/png,image/webp"
                        class="d-none"
                        data-testid="account-avatar-input"
                        @change="onAvatarSelected"
                    />
                </div>
                <v-btn
                    v-if="me.data.value?.avatar_url"
                    variant="text"
                    size="small"
                    color="error"
                    :loading="avatarBusy"
                    data-testid="account-avatar-remove"
                    @click="removeAvatar"
                >
                    {{ t('account.avatar.remove') }}
                </v-btn>
            </div>
            <v-form @submit.prevent="saveProfile">
                <v-text-field
                    v-model="displayName"
                    :label="t('account.profile.displayName')"
                    autocomplete="name"
                    data-testid="account-display-name"
                />
                <v-select
                    v-model="localeSelected"
                    :items="localeItems"
                    item-value="value"
                    item-title="title"
                    :label="t('language.label')"
                    data-testid="account-locale"
                />
                <v-btn type="submit" :loading="update.isPending.value" block class="mt-2" data-testid="account-save">
                    {{ t('account.profile.save') }}
                </v-btn>
            </v-form>

            <v-divider class="my-6" />

            <v-card-subtitle class="text-h6 px-0">{{ t('account.email.title') }}</v-card-subtitle>
            <p class="text-body-2 mb-2">
                {{ t('account.email.current') }}:
                <strong data-testid="account-email-current">{{ currentEmail }}</strong>
            </p>
            <v-alert
                v-if="changeRequested"
                type="success"
                variant="tonal"
                class="mb-4"
                data-testid="email-change-pending"
            >
                {{ t('account.email.pending') }}
            </v-alert>
            <v-form v-else @submit.prevent="submitEmailChange">
                <v-text-field
                    v-model="newEmail"
                    :label="t('account.email.newLabel')"
                    type="email"
                    autocomplete="email"
                    data-testid="account-email-new"
                />
                <v-btn
                    type="submit"
                    :loading="requestChange.isPending.value"
                    block
                    class="mt-2"
                    data-testid="account-email-change-submit"
                >
                    {{ t('account.email.submit') }}
                </v-btn>
            </v-form>

            <v-divider class="my-6" />

            <ReminderPrefsSection />

            <v-divider class="my-6" />

            <v-btn
                variant="outlined"
                color="error"
                block
                prepend-icon="log-out"
                data-testid="account-sign-out"
                @click="signOut"
            >
                {{ t('account.menu.signOut') }}
            </v-btn>
        </v-card>
    </v-container>
</template>

<style scoped>
.account-avatar-edit {
    position: absolute;
    bottom: -4px;
    right: -4px;
}
</style>
