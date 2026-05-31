<script setup lang="ts">
import { useQueryClient } from '@tanstack/vue-query'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useDisplay } from 'vuetify'

import { useMyFamilies } from '@/api/hooks/families'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'
import type { FamilyId } from '@/types/brand'
import { duplicateNameSet, formatFamilyDate } from '@/utils/familyDisplay'

// Sentinel value the v-select uses for the "Create new family…" trailing
// entry. A literal string is safer than `null` because v-model treats `null`
// as "no selection" and would render the field as empty after the click —
// breaking the visible-current-family invariant on the next reopen.
const CREATE_SENTINEL = '__create__'

const auth = useAuthStore()
const family = useActiveFamilyStore()
const locale = useLocaleStore()
const router = useRouter()
const queryClient = useQueryClient()
const { t } = useI18n()
// On phones the AppBar's title + sloth + avatar already eat most of the
// row width; a 220px-wide family-name selector pushes the avatar off
// the right edge or truncates the title. Collapse to an icon-only
// `users` button that opens the same family list via a v-menu when the
// viewport is small.
const { smAndDown } = useDisplay()

// Pull created_at per family from /families/me to disambiguate same-named
// families ONLY when the name actually repeats — unique names stay clean.
const myFamiliesQ = useMyFamilies()
const createdById = computed(() => {
    const map = new Map<string, string>()
    for (const m of myFamiliesQ.data.value ?? []) {
        map.set(m.id as string, m.created_at ?? '')
    }
    return map
})
const duplicates = computed(() => duplicateNameSet(auth.families))

const items = computed(() => {
    const familyItems = auth.families.map((f) => {
        const item: { value: string; title: string; props?: Record<string, unknown> } = {
            value: f.id as string,
            title: f.name,
        }
        if (duplicates.value.has(f.name)) {
            const date = formatFamilyDate(createdById.value.get(f.id as string), locale.locale)
            if (date !== null) {
                item.props = { subtitle: t('family.disambiguator', { date, role: f.role }) }
            }
        }
        return item
    })
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

// Vuetify's v-list-item props are strict (no `undefined` allowed when
// `exactOptionalPropertyTypes` is on). Build a plain bag of bindings
// per item that only includes optional fields when they're actually
// strings — then pass the bag via v-bind.
function bindingsFor(it: { value: string; title: string; props?: Record<string, unknown> }): Record<
    string,
    unknown
> {
    const bag: Record<string, unknown> = {
        active: it.value === family.activeFamilyId,
        title: it.title,
        onClick: () => {
            onChange(it.value)
        },
    }
    const prepend = it.props?.['prependIcon']
    if (typeof prepend === 'string') bag['prependIcon'] = prepend
    const subtitle = it.props?.['subtitle']
    if (typeof subtitle === 'string') bag['subtitle'] = subtitle
    return bag
}

function onChange(value: unknown): void {
    if (value === CREATE_SENTINEL) {
        void router.push('/families/create')
        return
    }
    if (typeof value !== 'string') return
    if (value === family.activeFamilyId) return
    family.setActive(value as FamilyId)
    // Refetch every query so the current view shows the new family's
    // data. invalidateQueries (no key filter) flips all caches to
    // stale + triggers refetch of any active observer — the client
    // middleware then re-issues each request with the new X-Family-Id
    // header. No page reload, no flicker, persisted Pinia state intact.
    void queryClient.invalidateQueries()
}
</script>

<template>
    <!-- Desktop / tablet: the wide v-select keeps the current family
         name visible inline. -->
    <v-select
        v-if="!smAndDown"
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
    <!-- Phone: collapse to a single icon-only activator. The `users`
         Lucide glyph reads as "family/group" and tapping it opens a
         v-menu of the exact same items the v-select shows on
         desktop, including the "Create new family…" trailing entry
         + divider. `data-testid="family-switcher"` is preserved so
         existing tests / e2e flows resolve to whichever variant is
         active for the current viewport. The activator's
         `aria-label` reads the active family name so screen readers
         keep parity with the desktop label. -->
    <v-menu v-else location="bottom end">
        <template #activator="{ props: activatorProps }">
            <v-btn
                v-bind="activatorProps"
                icon="users"
                variant="text"
                density="compact"
                :title="t('family.switcher.placeholder')"
                :aria-label="family.activeFamily?.name ?? placeholder"
                data-testid="family-switcher"
            />
        </template>
        <v-list density="compact" data-testid="family-switcher-menu">
            <!-- `'value' in it` discriminates the union: divider entries
                 have only `{ type: 'divider' }`, family + create entries
                 have `value`/`title`/`props`. TS narrows on this guard
                 the same way it does for `if (...) {} else {}`. -->
            <template v-for="(it, i) in items" :key="i">
                <v-list-item v-if="'value' in it" v-bind="bindingsFor(it)" />
                <v-divider v-else />
            </template>
        </v-list>
    </v-menu>
</template>
