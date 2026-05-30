<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { useRequestMagicLink } from '@/api/hooks/auth'

const { t } = useI18n()
const email = ref('')
const sent = ref(false)
const mutation = useRequestMagicLink()
const errorMsg = ref<string | null>(null)

async function submit(): Promise<void> {
    errorMsg.value = null
    try {
        await mutation.mutateAsync(email.value)
        sent.value = true
    } catch (e: unknown) {
        errorMsg.value = e instanceof Error ? e.message : 'unknown error'
    }
}
</script>

<template>
    <v-card class="pa-6" data-testid="login-card">
        <v-card-title class="text-h5 mb-2">{{ t('auth.signIn.title') }}</v-card-title>
        <v-card-subtitle class="text-body-1 mb-4">{{ t('auth.signIn.tagline') }}</v-card-subtitle>

        <v-alert v-if="errorMsg" type="error" class="mb-4" data-testid="login-error">
            {{ errorMsg }}
        </v-alert>

        <template v-if="!sent">
            <v-form @submit.prevent="submit">
                <v-text-field
                    v-model="email"
                    :label="t('auth.signIn.emailLabel')"
                    prepend-inner-icon="mail"
                    type="email"
                    required
                    autocomplete="email"
                    data-testid="sign-in-email"
                />
                <!--
                    Primary CTA on the page — `color="primary" variant="flat"`
                    gives it the signature sloth-orange fill. Vuetify's
                    `VBtn` defaults intentionally leave colour unset so
                    most buttons stay neutral; the page's CTA is the one
                    that opts in.
                -->
                <v-btn
                    type="submit"
                    color="primary"
                    variant="flat"
                    :loading="mutation.isPending.value"
                    block
                    size="large"
                    class="mt-3"
                    data-testid="sign-in-submit"
                >
                    {{ t('auth.signIn.send') }}
                </v-btn>
            </v-form>
        </template>

        <v-alert v-else type="success" data-testid="sign-in-sent" class="mt-2">
            <strong>{{ t('auth.signIn.sentTitle') }}</strong>
            <div class="text-body-2 mt-1">{{ t('auth.signIn.sent') }}</div>
        </v-alert>
    </v-card>
</template>
