<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { useAddParentLink, useDeleteParentLink } from '@/api/hooks/relationships'

export interface ParentRow {
    childId: string
    parentId: string
    kind: string
}

const props = defineProps<{
    personId: string
    canEdit: boolean
    rows: ParentRow[]
    otherItems: { value: string; title: string }[]
    labelFor: (id: string) => string
}>()

const emit = defineEmits<{
    (e: 'changed'): void
}>()

const { t } = useI18n()
const addParent = useAddParentLink()
const deleteParent = useDeleteParentLink()

const parentToAdd = ref<string | null>(null)
const parentKind = ref<'biological' | 'legal' | 'adoptive' | 'step' | 'social'>('biological')

const openKey = ref<string | null>(null)
const draftKind = ref<'biological' | 'legal' | 'adoptive' | 'step' | 'social' | null>(null)

const parentKindOptions = computed(() => [
    { value: 'biological', title: t('person.parentKind.biological') },
    { value: 'legal', title: t('person.parentKind.legal') },
    { value: 'adoptive', title: t('person.parentKind.adoptive') },
    { value: 'step', title: t('person.parentKind.step') },
    { value: 'social', title: t('person.parentKind.social') },
])

function rowKey(row: ParentRow): string {
    return `${row.childId}:${row.parentId}`
}

function startEditing(row: ParentRow): void {
    openKey.value = rowKey(row)
    draftKind.value = (row.kind as typeof draftKind.value) ?? 'biological'
}

async function save(row: ParentRow): Promise<void> {
    const next = draftKind.value
    if (next === null || next === row.kind) {
        openKey.value = null
        return
    }
    // No PATCH endpoint for parent_links — implement "change kind" as
    // DELETE + POST. On POST failure, best-effort restore the original
    // edge so the user's tree isn't left missing the relation.
    await deleteParent.mutateAsync({ child_id: row.childId, parent_id: row.parentId })
    try {
        await addParent.mutateAsync({
            child_id: row.childId,
            parent_id: row.parentId,
            kind: next,
        })
    } catch (err: unknown) {
        try {
            await addParent.mutateAsync({
                child_id: row.childId,
                parent_id: row.parentId,
                kind: row.kind,
            })
        } catch {
            // Rollback failure is logged via the global error toast — don't
            // mask the original error.
        }
        throw err
    }
    openKey.value = null
    draftKind.value = null
    emit('changed')
}

async function remove(row: ParentRow): Promise<void> {
    await deleteParent.mutateAsync({ child_id: row.childId, parent_id: row.parentId })
    openKey.value = null
    draftKind.value = null
    emit('changed')
}

async function linkParent(): Promise<void> {
    const pid = parentToAdd.value
    if (pid === null || pid === '') return
    await addParent.mutateAsync({
        child_id: props.personId,
        parent_id: pid,
        kind: parentKind.value,
    })
    parentToAdd.value = null
    parentKind.value = 'biological'
    emit('changed')
}

watch(
    () => props.personId,
    () => {
        openKey.value = null
        draftKind.value = null
    },
)
</script>

<template>
    <v-expansion-panel data-testid="relations-parents">
        <v-expansion-panel-title> {{ t('person.sections.parents') }} ({{ rows.length }}) </v-expansion-panel-title>
        <v-expansion-panel-text>
            <div
                v-for="row in rows"
                :key="rowKey(row)"
                class="relation-row"
                :data-testid="`relation-parent-${row.parentId}`"
            >
                <div class="d-flex align-center ga-2">
                    <span class="flex-grow-1">{{ labelFor(row.parentId) }}</span>
                    <v-chip size="x-small" variant="outlined">{{ row.kind }}</v-chip>
                    <v-btn
                        v-if="canEdit"
                        icon="edit"
                        size="x-small"
                        variant="text"
                        :data-testid="`relation-parent-edit-${row.parentId}`"
                        @click="startEditing(row)"
                    />
                </div>
                <div
                    v-if="canEdit && openKey === rowKey(row)"
                    class="inline-editor"
                    :data-testid="`relation-parent-editor-${row.parentId}`"
                >
                    <v-select
                        v-model="draftKind"
                        :items="parentKindOptions"
                        item-value="value"
                        item-title="title"
                        :label="t('person.fields.parentKind')"
                        density="comfortable"
                        :data-testid="`relation-parent-kind-${row.parentId}`"
                    />
                    <div class="d-flex justify-end ga-2">
                        <v-btn
                            size="small"
                            variant="text"
                            color="error"
                            :loading="deleteParent.isPending.value"
                            :data-testid="`relation-parent-delete-${row.parentId}`"
                            @click="remove(row)"
                        >
                            {{ t('person.actions.deleteRelation') }}
                        </v-btn>
                        <v-btn
                            size="small"
                            color="primary"
                            :loading="deleteParent.isPending.value || addParent.isPending.value"
                            :data-testid="`relation-parent-save-${row.parentId}`"
                            @click="save(row)"
                        >
                            {{ t('common.save') }}
                        </v-btn>
                    </div>
                </div>
            </div>
            <div v-if="canEdit" class="mt-2">
                <v-select
                    v-model="parentToAdd"
                    :items="otherItems"
                    item-value="value"
                    item-title="title"
                    :label="t('person.actions.addParent')"
                    clearable
                    density="comfortable"
                    data-testid="person-add-parent"
                />
                <v-select
                    v-model="parentKind"
                    :items="parentKindOptions"
                    item-value="value"
                    item-title="title"
                    :label="t('person.fields.parentKind')"
                    density="comfortable"
                    data-testid="person-add-parent-kind"
                />
                <v-btn
                    block
                    color="primary"
                    variant="tonal"
                    :disabled="parentToAdd === null || parentToAdd === ''"
                    :loading="addParent.isPending.value"
                    data-testid="person-add-parent-submit"
                    @click="linkParent"
                >
                    {{ t('common.add') }}
                </v-btn>
            </div>
        </v-expansion-panel-text>
    </v-expansion-panel>
</template>

<style scoped>
.relation-row {
    padding: 6px 0;
    border-bottom: 1px solid rgba(0, 0, 0, 0.08);
}
.relation-row:last-child {
    border-bottom: none;
}
.inline-editor {
    margin-top: 8px;
    padding: 8px;
    background: rgba(0, 0, 0, 0.03);
    border-radius: 6px;
}
</style>
