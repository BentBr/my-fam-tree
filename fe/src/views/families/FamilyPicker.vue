<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useMyFamilies } from '@/api/hooks/families'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'
import type { FamilyId } from '@/types/brand'
import { duplicateNameSet, formatFamilyDate } from '@/utils/familyDisplay'

const { t } = useI18n()
const auth = useAuthStore()
const family = useActiveFamilyStore()
const locale = useLocaleStore()
const router = useRouter()

// Pull `created_at` per family from /families/me (the JWT claim deliberately
// omits it). `auth.families` is the instant source for names/roles, and we
// augment with the timestamp once the query lands.
const myFamiliesQ = useMyFamilies()
const createdById = computed(() => {
    const map = new Map<string, string>()
    for (const m of myFamiliesQ.data.value ?? []) {
        map.set(m.id as string, m.created_at ?? '')
    }
    return map
})

const duplicates = computed(() => duplicateNameSet(auth.families))

function subtitleOf(f: { id: string; name: string; role: string }): string {
    if (!duplicates.value.has(f.name)) return f.role
    const date = formatFamilyDate(createdById.value.get(f.id), locale.locale)
    if (date === null) return f.role
    return t('family.disambiguator', { date, role: f.role })
}

async function pick(id: FamilyId): Promise<void> {
    family.setActive(id)
    // Land on the main app view after picking, not Health (which is now a
    // drawer footnote, not a primary route).
    await router.replace('/tree')
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
                :subtitle="subtitleOf({ id: f.id as string, name: f.name, role: f.role })"
                @click="pick(f.id)"
            />
        </v-list>
        <v-btn to="/families/create" variant="text" class="mt-2" data-testid="family-picker-create">
            {{ t('family.picker.create') }}
        </v-btn>
    </v-card>
</template>
