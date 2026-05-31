<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { useFamilyOverview, useRenameFamily } from '@/api/hooks/families'

const { t, locale } = useI18n()
const q = useFamilyOverview()
const rename = useRenameFamily()

// "Edit name" UI state. The page renders the current name as static
// text until the user clicks Rename — then it swaps to a v-text-field
// pre-filled with the current value. Save commits via the mutation;
// Cancel reverts without firing anything.
const editing = ref(false)
const draftName = ref('')

// `useFamilyOverview` already calls `unwrap()` internally, so `q.data.value`
// is the inner `FamilyOverview` (not the `{ data, meta }` envelope).
const family = computed(() => q.data.value ?? null)

watch(family, (next) => {
    if (next !== null && !editing.value) {
        draftName.value = next.name
    }
})

function startEdit(): void {
    if (family.value === null) return
    draftName.value = family.value.name
    editing.value = true
}

function cancelEdit(): void {
    editing.value = false
    if (family.value !== null) draftName.value = family.value.name
}

async function saveEdit(): Promise<void> {
    if (family.value === null) return
    const trimmed = draftName.value.trim()
    if (trimmed === '' || trimmed === family.value.name) {
        editing.value = false
        return
    }
    await rename.mutateAsync({ id: family.value.id, name: trimmed })
    editing.value = false
}

// Localized date formatter for the "added on" stamps in the latest-
// persons rail. Same locale binding as the rest of the admin pages.
function fmtDate(iso: string): string {
    try {
        return new Date(iso).toLocaleDateString(locale.value)
    } catch {
        return iso
    }
}

// "Open in tree" → /tree?center=<personId>. TreeView's existing
// center-on-member handler picks up the query param and centers the
// canvas on that node.
function centerHref(personId: string): string {
    return `/tree?center=${encodeURIComponent(personId)}`
}

function fullName(p: { given_name: string; family_name: string }): string {
    return [p.given_name, p.family_name].filter((s) => s !== '').join(' ')
}
</script>

<template>
    <section class="family-page" data-testid="admin-family-page">
        <header class="d-flex align-center mb-3">
            <h2 class="text-h6">{{ t('admin.family.title') }}</h2>
        </header>

        <v-skeleton-loader v-if="q.isLoading.value" type="card" data-testid="admin-family-loading" />
        <v-alert v-else-if="q.error.value !== null" type="error" data-testid="admin-family-error">
            {{ t('errors.generic') }}
        </v-alert>

        <template v-else-if="family !== null">
            <!-- Name + rename. View mode shows the family name as a
                 heading with a small "Rename" pencil action; edit
                 mode swaps in a single v-text-field with Save / Cancel
                 actions inline. Both modes carry the same testid so
                 e2e + unit tests can locate the row regardless of
                 which mode it's in. -->
            <v-card variant="outlined" class="mb-4" data-testid="admin-family-card">
                <v-card-title class="d-flex align-center" style="gap: 0.5rem">
                    <span class="text-overline">{{ t('admin.family.nameLabel') }}</span>
                </v-card-title>
                <v-card-text>
                    <div v-if="!editing" class="d-flex align-center" style="gap: 0.75rem">
                        <span class="text-h5" data-testid="admin-family-name">{{ family.name }}</span>
                        <v-btn
                            size="small"
                            variant="text"
                            prepend-icon="pencil"
                            data-testid="admin-family-rename"
                            @click="startEdit"
                        >
                            {{ t('admin.family.rename') }}
                        </v-btn>
                    </div>
                    <div v-else class="d-flex align-center" style="gap: 0.5rem">
                        <v-text-field
                            v-model="draftName"
                            :label="t('admin.family.nameLabel')"
                            density="compact"
                            hide-details
                            autofocus
                            data-testid="admin-family-name-input"
                            @keydown.enter="saveEdit"
                            @keydown.escape="cancelEdit"
                        />
                        <v-btn
                            color="primary"
                            variant="flat"
                            size="small"
                            :loading="rename.isPending.value"
                            data-testid="admin-family-name-save"
                            @click="saveEdit"
                        >
                            {{ t('common.save') }}
                        </v-btn>
                        <v-btn
                            variant="text"
                            size="small"
                            data-testid="admin-family-name-cancel"
                            @click="cancelEdit"
                        >
                            {{ t('common.cancel') }}
                        </v-btn>
                    </div>
                </v-card-text>
            </v-card>

            <!-- Two stat tiles side-by-side: member count linking to
                 the members admin page; person count linking to the
                 tree. d-flex with wrap so the tiles stack on narrow
                 viewports without overflow. -->
            <div class="stat-tiles mb-4">
                <v-card
                    variant="outlined"
                    class="stat-tile"
                    to="/admin/members"
                    data-testid="admin-family-members-tile"
                >
                    <v-card-text>
                        <div class="text-overline">{{ t('admin.family.members') }}</div>
                        <div class="text-h4" data-testid="admin-family-member-count">
                            {{ family.member_count }}
                        </div>
                        <div class="text-caption text-medium-emphasis">
                            {{ t('admin.family.membersOpen') }}
                        </div>
                    </v-card-text>
                </v-card>
                <v-card variant="outlined" class="stat-tile" to="/tree" data-testid="admin-family-persons-tile">
                    <v-card-text>
                        <div class="text-overline">{{ t('admin.family.persons') }}</div>
                        <div class="text-h4" data-testid="admin-family-person-count">
                            {{ family.person_count }}
                        </div>
                        <div class="text-caption text-medium-emphasis">
                            {{ t('admin.family.personsOpen') }}
                        </div>
                    </v-card-text>
                </v-card>
            </div>

            <v-card variant="outlined" data-testid="admin-family-latest">
                <v-card-title>{{ t('admin.family.latestTitle') }}</v-card-title>
                <v-list density="compact">
                    <v-list-item
                        v-if="family.latest_persons.length === 0"
                        data-testid="admin-family-latest-empty"
                    >
                        {{ t('admin.family.latestEmpty') }}
                    </v-list-item>
                    <v-list-item
                        v-for="p in family.latest_persons"
                        :key="p.id"
                        :to="centerHref(p.id)"
                        :prepend-icon="'user'"
                        :title="fullName(p)"
                        :subtitle="t('admin.family.addedOn', { date: fmtDate(p.created_at) })"
                        :data-testid="`admin-family-latest-${p.id}`"
                    />
                </v-list>
            </v-card>
        </template>
    </section>
</template>

<style scoped>
.family-page {
    max-width: 960px;
}
.stat-tiles {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
}
.stat-tile {
    flex: 1 1 240px;
    min-width: 220px;
    text-decoration: none;
}
</style>
