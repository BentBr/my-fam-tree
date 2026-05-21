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
    return [
        ...familyItems,
        // Divider before the create entry so it visually separates from the
        // family list. v-select honours `props: { divider: true }` on item
        // objects via the `header` slot, but the simpler portable approach
        // is a `type: 'divider'` item that v-select renders as a divider.
        { type: 'divider' as const },
        {
            value: CREATE_SENTINEL,
            title: t('family.switcher.createNew'),
            // `props` is forwarded to the underlying v-list-item, so we can
            // give the create entry a distinct icon without a custom slot.
            props: { prependIcon: 'plus' },
        },
    ]
})

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
