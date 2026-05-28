<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useCreateFamily } from '@/api/hooks/families'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import type { FamilyId } from '@/types/brand'

const { t } = useI18n()
const name = ref('')
const router = useRouter()
const create = useCreateFamily()
const family = useActiveFamilyStore()
const auth = useAuthStore()
const errorMsg = ref<string | null>(null)

// Duplicates are deliberately allowed by the backend (no UNIQUE constraint on
// families.name), so the picker/switcher disambiguate by created-date. Here
// we only warn when the user is creating ANOTHER family they already OWN —
// a member-of-Peters creating their own Peters is the common, intentional
// case (e.g., a different branch of the same surname) and shouldn't nag.
const ownedNames = computed(() => {
    const owned = new Set<string>()
    for (const f of auth.families) {
        if (f.role === 'owner') owned.add(f.name)
    }
    return owned
})

const showDupDialog = ref(false)
const pendingName = ref('')

async function submit(): Promise<void> {
    const trimmed = name.value.trim()
    if (trimmed === '') return
    if (ownedNames.value.has(trimmed)) {
        pendingName.value = trimmed
        showDupDialog.value = true
        return
    }
    await actuallyCreate(trimmed)
}

async function actuallyCreate(n: string): Promise<void> {
    errorMsg.value = null
    try {
        const res = await create.mutateAsync(n)
        if (res !== undefined) {
            family.setActive(res.family.id as FamilyId)
            // Flush reactivity so the family guard reads activeFamilyId BEFORE
            // evaluating /tree — otherwise it can bounce back.
            await nextTick()
            await router.push('/tree')
        }
    } catch (e: unknown) {
        errorMsg.value = e instanceof Error ? e.message : 'unknown error'
    }
}

async function confirmDuplicate(): Promise<void> {
    showDupDialog.value = false
    const n = pendingName.value
    pendingName.value = ''
    await actuallyCreate(n)
}

function cancelDuplicate(): void {
    showDupDialog.value = false
    pendingName.value = ''
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

        <!-- Soft confirm: same-named families are allowed, but creating a
             second one of your OWN deserves an extra click of intent. -->
        <v-dialog v-model="showDupDialog" max-width="500" data-testid="family-duplicate-dialog">
            <v-card>
                <v-card-title>{{ t('family.duplicateConfirm.title', { name: pendingName }) }}</v-card-title>
                <v-card-text>{{ t('family.duplicateConfirm.body', { name: pendingName }) }}</v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" data-testid="family-duplicate-cancel" @click="cancelDuplicate">
                        {{ t('family.duplicateConfirm.cancel') }}
                    </v-btn>
                    <v-btn color="primary" data-testid="family-duplicate-confirm" @click="confirmDuplicate">
                        {{ t('family.duplicateConfirm.confirm') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </v-card>
</template>
