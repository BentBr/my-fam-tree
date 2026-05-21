<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useCreateFamily } from '@/api/hooks/families'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import type { FamilyId } from '@/types/brand'

const { t } = useI18n()
const name = ref('')
const router = useRouter()
const create = useCreateFamily()
const family = useActiveFamilyStore()
const errorMsg = ref<string | null>(null)

async function submit(): Promise<void> {
    errorMsg.value = null
    try {
        const res = await create.mutateAsync(name.value.trim())
        if (res !== undefined) {
            family.setActive(res.data.family.id as FamilyId)
            // Flush reactivity so the family guard reads activeFamilyId
            // BEFORE evaluating /health — otherwise it can bounce back.
            await nextTick()
            await router.push('/health')
        }
    } catch (e: unknown) {
        errorMsg.value = e instanceof Error ? e.message : 'unknown error'
    }
}
</script>

<template>
    <v-card class="pa-6" data-testid="family-create-card">
        <v-card-title class="text-h5 mb-2">{{ t('family.create.title') }}</v-card-title>
        <v-alert v-if="errorMsg" type="error" class="mb-4" data-testid="family-create-error">
            {{ errorMsg }}
        </v-alert>
        <v-form @submit.prevent="submit">
            <v-text-field
                v-model="name"
                :label="t('family.create.nameLabel')"
                required
                autocomplete="off"
                data-testid="family-name"
            />
            <v-btn
                type="submit"
                :loading="create.isPending.value"
                block
                size="large"
                class="mt-3"
                data-testid="family-create-submit"
            >
                {{ t('family.create.submit') }}
            </v-btn>
        </v-form>
    </v-card>
</template>
