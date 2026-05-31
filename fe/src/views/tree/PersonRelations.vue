<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { useListPersons } from '@/api/hooks/persons'
import { useTree } from '@/api/hooks/relationships'

import RelationsParentPanel from './RelationsParentPanel.vue'
import type { ParentRow } from './RelationsParentPanel.vue'
import RelationsPartnerPanel from './RelationsPartnerPanel.vue'
import type { PartnerRow } from './RelationsPartnerPanel.vue'

const props = defineProps<{
    personId: string
    canEdit: boolean
}>()

const emit = defineEmits<{
    (e: 'changed'): void
}>()

const { t } = useI18n()
const list = useListPersons()
const tree = useTree()

// Lookup table to resolve relation ids → display name. Falls back to the id
// when a person isn't in the cached list (unlikely; tree + list share the
// same family scope).
function personLabel(id: string): string {
    const p = list.data.value?.find((x) => x.id === id)
    if (p === undefined) return id
    const full = `${p.given_name} ${p.family_name}`.trim()
    return full !== '' ? full : p.given_name
}

// Partner edges for this person — find rows where `a` or `b` matches the
// current person, then normalise into "the OTHER partner" view.
const partnerRows = computed<PartnerRow[]>(() => {
    const edges = tree.data.value?.partner_edges ?? []
    return edges
        .filter((e) => e.a === props.personId || e.b === props.personId)
        .map((e) => ({
            id: e.id,
            otherId: e.a === props.personId ? e.b : e.a,
            kind: e.kind,
            started_on: e.started_on ?? null,
            ended_on: e.ended_on ?? null,
            end_reason: e.end_reason ?? null,
        }))
})

const parentRows = computed<ParentRow[]>(() => {
    const edges = tree.data.value?.parent_edges ?? []
    return edges.filter((e) => e.a === props.personId).map((e) => ({ childId: e.a, parentId: e.b, kind: e.kind }))
})

const childRows = computed<ParentRow[]>(() => {
    const edges = tree.data.value?.parent_edges ?? []
    return edges.filter((e) => e.b === props.personId).map((e) => ({ childId: e.a, parentId: e.b, kind: e.kind }))
})

const otherItems = computed(() =>
    (list.data.value ?? [])
        .filter((p) => p.id !== props.personId)
        .map((p) => ({
            value: p.id,
            title: `${p.given_name} ${p.family_name}`.trim() || p.given_name,
        })),
)

function onChanged(): void {
    emit('changed')
}
</script>

<template>
    <v-expansion-panels variant="accordion" data-testid="person-relations">
        <RelationsParentPanel
            :person-id="props.personId"
            :can-edit="canEdit"
            :rows="parentRows"
            :other-items="otherItems"
            :label-for="personLabel"
            @changed="onChanged"
        />
        <RelationsPartnerPanel
            :person-id="props.personId"
            :can-edit="canEdit"
            :rows="partnerRows"
            :other-items="otherItems"
            :label-for="personLabel"
            @changed="onChanged"
        />
        <v-expansion-panel data-testid="relations-children">
            <v-expansion-panel-title>
                {{ t('person.sections.children') }} ({{ childRows.length }})
            </v-expansion-panel-title>
            <v-expansion-panel-text>
                <!-- Children are inherently parent-links from the other side.
                     Editing each parent-link belongs on the child person's own
                     drawer (where it's clear which child's relationship is
                     being modified), so this panel is display-only. -->
                <div
                    v-for="row in childRows"
                    :key="`${row.childId}:${row.parentId}`"
                    class="relation-row"
                    :data-testid="`relation-child-${row.childId}`"
                >
                    <div class="d-flex align-center ga-2">
                        <span class="flex-grow-1">{{ personLabel(row.childId) }}</span>
                        <v-chip size="x-small" variant="outlined">{{ row.kind }}</v-chip>
                    </div>
                </div>
            </v-expansion-panel-text>
        </v-expansion-panel>
    </v-expansion-panels>
</template>

<style scoped>
.relation-row {
    padding: 6px 0;
    border-bottom: 1px solid rgba(0, 0, 0, 0.08);
}
.relation-row:last-child {
    border-bottom: none;
}

/* `variant="accordion"` stacks panels with no gap — the closed-state
   titles end up shoulder-to-shoulder (image 33) which reads as one
   blob rather than three distinct sections. A small gap separates
   them visually without touching the inner accordion behaviour. */
:deep(.v-expansion-panel) {
    margin-bottom: 8px;
}
:deep(.v-expansion-panel:last-child) {
    margin-bottom: 0;
}
</style>
