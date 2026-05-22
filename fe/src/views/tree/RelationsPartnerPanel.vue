<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { useCreatePartnership, useDeletePartnership, useUpdatePartnership } from '@/api/hooks/relationships'

export interface PartnerRow {
    id: string
    otherId: string
    kind: string
    started_on: string | null
    ended_on: string | null
    end_reason: string | null
}

const props = defineProps<{
    personId: string
    canEdit: boolean
    rows: PartnerRow[]
    otherItems: { value: string; title: string }[]
    labelFor: (id: string) => string
}>()

const emit = defineEmits<{
    (e: 'changed'): void
}>()

const { t } = useI18n()
const createPartner = useCreatePartnership()
const updatePartner = useUpdatePartnership()
const deletePartner = useDeletePartnership()

const partnerToAdd = ref<string | null>(null)
const partnerKind = ref<'marriage' | 'civil_union' | 'partnership' | null>(null)
const partnerStartedOn = ref<string | null>(null)

const openId = ref<string | null>(null)

interface PartnerDraft {
    kind: 'marriage' | 'civil_union' | 'partnership'
    started_on: string | null
    ended_on: string | null
    end_reason: 'divorce' | 'separation' | 'death' | null
}
const draft = ref<PartnerDraft | null>(null)

const partnerKindOptions = computed(() => [
    { value: 'marriage', title: t('person.partnerKind.marriage') },
    { value: 'civil_union', title: t('person.partnerKind.civil_union') },
    { value: 'partnership', title: t('person.partnerKind.partnership') },
])
const endReasonOptions = computed(() => [
    { value: 'divorce', title: t('person.endReason.divorce') },
    { value: 'separation', title: t('person.endReason.separation') },
    { value: 'death', title: t('person.endReason.death') },
])

// YYYY-MM-DD in local time — matches `<input type="date">` format and
// sidesteps the timezone-shifted-by-a-day surprise of `toISOString()`.
function todayIso(): string {
    const d = new Date()
    const year = d.getFullYear()
    const month = String(d.getMonth() + 1).padStart(2, '0')
    const day = String(d.getDate()).padStart(2, '0')
    return `${year}-${month}-${day}`
}

function startEditing(row: PartnerRow): void {
    openId.value = row.id
    draft.value = {
        kind: (row.kind as PartnerDraft['kind']) ?? 'partnership',
        started_on: row.started_on,
        ended_on: row.ended_on,
        end_reason: row.end_reason as PartnerDraft['end_reason'] | null,
    }
}

function endPartnership(row: PartnerRow): void {
    // Pre-fill `ended_on = today` and `end_reason = 'divorce'`. The user can
    // still adjust before pressing Save.
    openId.value = row.id
    draft.value = {
        kind: (row.kind as PartnerDraft['kind']) ?? 'partnership',
        started_on: row.started_on,
        ended_on: todayIso(),
        end_reason: 'divorce',
    }
}

async function save(row: PartnerRow): Promise<void> {
    const d = draft.value
    if (d === null) return
    // Diff against the row — empty-body PATCHes return
    // `validation.value_required`, so bail if nothing changed.
    const input: {
        kind?: string
        started_on?: string | null
        ended_on?: string | null
        end_reason?: string | null
    } = {}
    if (d.kind !== row.kind) input.kind = d.kind
    if ((d.started_on ?? null) !== (row.started_on ?? null)) {
        input.started_on = d.started_on === '' ? null : d.started_on
    }
    if ((d.ended_on ?? null) !== (row.ended_on ?? null)) {
        input.ended_on = d.ended_on === '' ? null : d.ended_on
    }
    if ((d.end_reason ?? null) !== (row.end_reason ?? null)) {
        input.end_reason = d.end_reason ?? ''
    }
    if (Object.keys(input).length === 0) {
        openId.value = null
        draft.value = null
        return
    }
    await updatePartner.mutateAsync({ id: row.id, input })
    openId.value = null
    draft.value = null
    emit('changed')
}

async function remove(row: PartnerRow): Promise<void> {
    await deletePartner.mutateAsync(row.id)
    openId.value = null
    draft.value = null
    emit('changed')
}

async function linkPartner(): Promise<void> {
    const pid = partnerToAdd.value
    const kind = partnerKind.value
    if (pid === null || pid === '' || kind === null) return
    const started = partnerStartedOn.value
    const payload: {
        partner_a_id: string
        partner_b_id: string
        kind: typeof kind
        started_on?: string
    } = {
        partner_a_id: props.personId,
        partner_b_id: pid,
        kind,
    }
    if (started !== null && started !== '') payload.started_on = started
    await createPartner.mutateAsync(payload)
    partnerToAdd.value = null
    partnerKind.value = null
    partnerStartedOn.value = null
    emit('changed')
}

watch(
    () => props.personId,
    () => {
        openId.value = null
        draft.value = null
    },
)
</script>

<template>
    <v-expansion-panel data-testid="relations-partners">
        <v-expansion-panel-title> {{ t('person.sections.partners') }} ({{ rows.length }}) </v-expansion-panel-title>
        <v-expansion-panel-text>
            <div v-for="row in rows" :key="row.id" class="relation-row" :data-testid="`relation-partner-${row.id}`">
                <div class="d-flex align-center ga-2">
                    <span class="flex-grow-1">{{ labelFor(row.otherId) }}</span>
                    <v-chip size="x-small" variant="outlined">{{ row.kind }}</v-chip>
                    <v-chip
                        v-if="row.ended_on !== null"
                        size="x-small"
                        color="grey-lighten-1"
                        variant="tonal"
                        :data-testid="`relation-partner-ended-${row.id}`"
                    >
                        {{ row.ended_on }}
                    </v-chip>
                    <v-btn
                        v-if="canEdit"
                        icon="edit"
                        size="x-small"
                        variant="text"
                        :data-testid="`relation-partner-edit-${row.id}`"
                        @click="startEditing(row)"
                    />
                    <v-btn
                        v-if="canEdit && row.ended_on === null"
                        size="x-small"
                        variant="outlined"
                        color="warning"
                        :data-testid="`relation-partner-end-${row.id}`"
                        @click="endPartnership(row)"
                    >
                        {{ t('person.actions.endPartnership') }}
                    </v-btn>
                </div>
                <div
                    v-if="canEdit && openId === row.id && draft !== null"
                    class="inline-editor"
                    :data-testid="`relation-partner-editor-${row.id}`"
                >
                    <v-select
                        v-model="draft.kind"
                        :items="partnerKindOptions"
                        item-value="value"
                        item-title="title"
                        :label="t('person.fields.partnerKind')"
                        density="comfortable"
                        :data-testid="`relation-partner-kind-${row.id}`"
                    />
                    <v-text-field
                        v-model="draft.started_on"
                        :label="t('person.fields.started_on')"
                        type="date"
                        density="comfortable"
                        clearable
                        :data-testid="`relation-partner-started-on-${row.id}`"
                    />
                    <v-text-field
                        v-model="draft.ended_on"
                        :label="t('person.fields.ended_on')"
                        type="date"
                        density="comfortable"
                        clearable
                        :data-testid="`relation-partner-ended-on-${row.id}`"
                    />
                    <v-select
                        v-model="draft.end_reason"
                        :items="endReasonOptions"
                        item-value="value"
                        item-title="title"
                        :label="t('person.fields.end_reason')"
                        density="comfortable"
                        clearable
                        :data-testid="`relation-partner-end-reason-${row.id}`"
                    />
                    <div class="d-flex justify-end ga-2">
                        <v-btn
                            size="small"
                            variant="text"
                            color="error"
                            :loading="deletePartner.isPending.value"
                            :data-testid="`relation-partner-delete-${row.id}`"
                            @click="remove(row)"
                        >
                            {{ t('person.actions.deleteRelation') }}
                        </v-btn>
                        <v-btn
                            size="small"
                            color="primary"
                            :loading="updatePartner.isPending.value"
                            :data-testid="`relation-partner-save-${row.id}`"
                            @click="save(row)"
                        >
                            {{ t('common.save') }}
                        </v-btn>
                    </div>
                </div>
            </div>
            <div v-if="canEdit" class="mt-2">
                <v-select
                    v-model="partnerToAdd"
                    :items="otherItems"
                    item-value="value"
                    item-title="title"
                    :label="t('person.actions.addPartner')"
                    clearable
                    density="comfortable"
                    data-testid="person-add-partner"
                />
                <v-select
                    v-model="partnerKind"
                    :items="partnerKindOptions"
                    item-value="value"
                    item-title="title"
                    :label="t('person.fields.partnerKind')"
                    density="comfortable"
                    clearable
                    data-testid="person-add-partner-kind"
                />
                <v-text-field
                    v-model="partnerStartedOn"
                    :label="t('person.fields.started_on')"
                    type="date"
                    density="comfortable"
                    clearable
                    data-testid="person-add-partner-started-on"
                />
                <v-btn
                    block
                    color="primary"
                    variant="tonal"
                    :disabled="partnerToAdd === null || partnerToAdd === '' || partnerKind === null"
                    :loading="createPartner.isPending.value"
                    data-testid="person-add-partner-submit"
                    @click="linkPartner"
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
