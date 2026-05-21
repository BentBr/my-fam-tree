<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import type { FamilyId } from '@/types/brand'

const { t } = useI18n()
const auth = useAuthStore()
const family = useActiveFamilyStore()
const router = useRouter()

async function pick(id: FamilyId): Promise<void> {
    family.setActive(id)
    await router.replace('/health')
}
</script>

<template>
    <v-card class="pa-6" data-testid="family-picker-card">
        <v-card-title class="text-h5 mb-2">{{ t('family.picker.title') }}</v-card-title>
        <v-list density="comfortable" nav>
            <v-list-item
                v-for="f in auth.families"
                :key="f.id"
                :data-testid="`pick-${f.id}`"
                :title="f.name"
                :subtitle="f.role"
                @click="pick(f.id)"
            />
        </v-list>
        <v-btn to="/families/create" variant="text" class="mt-2" data-testid="family-picker-create">
            {{ t('family.picker.create') }}
        </v-btn>
    </v-card>
</template>
