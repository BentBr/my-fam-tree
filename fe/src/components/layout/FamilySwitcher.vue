<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import type { FamilyId } from '@/types/brand'

// Sentinel value the v-select uses for the "Create new family…" trailing
// entry. A literal string is safer than `null` because v-model treats `null`
// as "no selection" and would render the field as empty after the click —
// breaking the visible-current-family invariant on the next reopen.
const CREATE_SENTINEL = '__create__'

const auth = useAuthStore()
const family = useActiveFamilyStore()
const router = useRouter()
const { t } = useI18n()

const items = computed(() => {
    const familyItems = auth.families.map((f) => ({
        value: f.id as string,
        title: f.name,
    }))
    const createEntry = {
        value: CREATE_SENTINEL,
        title: t('family.switcher.createNew'),
        // `props` is forwarded to the underlying v-list-item, so we can give
        // the create entry a distinct icon without a custom slot.
        props: { prependIcon: 'plus' },
    }
    // Empty-family case: only the create entry, no divider (nothing to separate).
    // The picker visibly invites the user to bootstrap their first family
    // rather than disappearing entirely — keeps the AppBar geometry stable
    // and gives a one-click path to /families/create.
    if (familyItems.length === 0) {
        return [createEntry]
    }
    return [
        ...familyItems,
        // Divider before the create entry so it visually separates from the
        // family list. `type: 'divider'` is rendered by v-select as a divider.
        { type: 'divider' as const },
        createEntry,
    ]
})

// Placeholder shown when the active family is null (e.g. before a 1-family
// user gets auto-picked, or when they have zero families). Localized.
const placeholder = computed(() => t('family.switcher.placeholder'))

function onChange(value: unknown): void {
    if (value === CREATE_SENTINEL) {
        void router.push('/families/create')
        return
    }
    if (typeof value === 'string') family.setActive(value as FamilyId)
}
</script>

<template>
    <v-select
        :model-value="family.activeFamilyId"
        :items="items"
        :placeholder="placeholder"
        item-value="value"
        item-title="title"
        density="compact"
        hide-details
        style="max-width: 220px"
        data-testid="family-switcher"
        @update:model-value="onChange"
    />
</template>
