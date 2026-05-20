<script setup lang="ts">
import { computed } from 'vue'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import type { FamilyId } from '@/types/brand'

const auth = useAuthStore()
const family = useActiveFamilyStore()
const items = computed(() => auth.families.map((f) => ({ value: f.id, title: f.name })))

function onChange(value: unknown): void {
    if (typeof value === 'string') family.setActive(value as FamilyId)
}
</script>

<template>
    <v-select
        v-if="auth.families.length > 0"
        :model-value="family.activeFamilyId"
        :items="items"
        item-value="value"
        item-title="title"
        density="compact"
        hide-details
        style="max-width: 220px"
        data-testid="family-switcher"
        @update:model-value="onChange"
    />
</template>
