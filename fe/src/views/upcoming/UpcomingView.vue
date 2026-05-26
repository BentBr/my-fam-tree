<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { type UpcomingFilter, useUpcoming } from '@/api/hooks/upcoming'

const { t, locale } = useI18n()
const router = useRouter()

// The toolbar shows two toggle buttons. Selecting one switches the
// filter to that mode; clicking the selected button again reverts to
// `all` (the default). Both unselected ⇒ `all`. We model this as a
// single ref rather than two booleans so the "exactly one selected
// or none" invariant is impossible to break.
const filter = ref<UpcomingFilter>('all')
function toggle(target: 'birthday' | 'anniversary'): void {
    filter.value = filter.value === target ? 'all' : target
}

// Third pill: "Favourites only". Per-user; the BE resolves the mark set
// against the signed-in caller. AND-composes with the kind filter — you
// can ask for "favourite + birthday" simultaneously.
const favouritesOnly = ref(false)
function toggleFavourites(): void {
    favouritesOnly.value = !favouritesOnly.value
}

const query = useUpcoming(filter, favouritesOnly)

const iconFor: Record<string, string> = {
    birthday: 'cake',
    wedding_anniversary: 'heart',
    // Latin cross (✝) is rendered by `LucideIcon` from a custom inline
    // SVG — lucide itself ships only the `+`-shaped medical `cross`,
    // which reads wrong for a memorial entry in the German Christian
    // convention the seed family lives in.
    death_anniversary: 'latin-cross',
}

const dateFormatter = computed(
    () =>
        new Intl.DateTimeFormat(locale.value, {
            weekday: 'short',
            year: 'numeric',
            month: 'short',
            day: 'numeric',
        }),
)

interface UpcomingRow {
    kind: string
    next_date: string
    years: number
    person_id?: string | null
    partnership_id?: string | null
    partner_a_id?: string | null
    partner_b_id?: string | null
    label: string
}

const rows = computed<UpcomingRow[]>(() => (query.data.value as UpcomingRow[] | undefined) ?? [])

function formatDate(iso: string): string {
    const d = new Date(`${iso}T00:00:00`)
    return dateFormatter.value.format(d)
}

function daysUntil(iso: string): number {
    const today = new Date()
    today.setHours(0, 0, 0, 0)
    const target = new Date(`${iso}T00:00:00`)
    const ms = target.getTime() - today.getTime()
    return Math.round(ms / (24 * 60 * 60 * 1000))
}

function relative(iso: string): string {
    const d = daysUntil(iso)
    if (d === 0) return t('upcoming.relative.today')
    if (d === 1) return t('upcoming.relative.tomorrow')
    return t('upcoming.relative.inDays', { count: d })
}

function onRowClick(row: UpcomingRow): void {
    // Birthdays + death anniversaries carry `person_id` directly.
    // Wedding anniversaries don't, but the BE attaches both partner
    // ids — we center on partner_a so the user lands on a real
    // PersonDetail; the tree edge to partner_b shows the relation.
    const targetId =
        (row.person_id !== null && row.person_id !== undefined && row.person_id !== '' ? row.person_id : null) ??
        (row.partner_a_id !== null && row.partner_a_id !== undefined && row.partner_a_id !== ''
            ? row.partner_a_id
            : null)
    if (targetId !== null) {
        void router.push({ path: '/tree', query: { center: targetId } })
    }
}
</script>

<template>
    <div class="upcoming-page" data-testid="upcoming-page">
        <v-toolbar density="comfortable" elevation="0" color="transparent">
            <v-toolbar-title>{{ t('upcoming.title') }}</v-toolbar-title>
            <v-spacer />
            <!-- Two plain v-btns rather than a v-btn-toggle: the user
                 wants "click → on; click again → off (back to all)"
                 which v-btn-toggle's `mandatory` modes don't model
                 cleanly. The `variant` prop drives the visual state. -->
            <div class="filter-buttons" data-testid="upcoming-filter">
                <v-btn
                    :variant="filter === 'birthday' ? 'flat' : 'tonal'"
                    :color="filter === 'birthday' ? 'primary' : undefined"
                    prepend-icon="cake"
                    rounded="lg"
                    density="comfortable"
                    data-testid="upcoming-filter-birthday"
                    @click="toggle('birthday')"
                >
                    {{ t('upcoming.filter.birthday') }}
                </v-btn>
                <v-btn
                    :variant="filter === 'anniversary' ? 'flat' : 'tonal'"
                    :color="filter === 'anniversary' ? 'primary' : undefined"
                    prepend-icon="heart"
                    rounded="lg"
                    density="comfortable"
                    data-testid="upcoming-filter-anniversary"
                    @click="toggle('anniversary')"
                >
                    {{ t('upcoming.filter.anniversary') }}
                </v-btn>
                <v-btn
                    :variant="favouritesOnly ? 'flat' : 'tonal'"
                    :color="favouritesOnly ? 'primary' : undefined"
                    prepend-icon="star"
                    rounded="lg"
                    density="comfortable"
                    data-testid="upcoming-filter-favourites"
                    @click="toggleFavourites"
                >
                    {{ t('upcoming.filter.favourites') }}
                </v-btn>
            </div>
        </v-toolbar>

        <v-skeleton-loader v-if="query.isLoading.value" type="list-item-three-line" />
        <v-alert v-else-if="query.error.value" type="error" data-testid="upcoming-error">
            {{ t('errors.generic') }}
        </v-alert>
        <div v-else-if="rows.length === 0" class="empty-state" data-testid="upcoming-empty">
            <v-card variant="tonal" class="pa-6">
                <v-card-title>{{ t('upcoming.empty.title') }}</v-card-title>
                <v-card-text>{{ t('upcoming.empty.subtitle') }}</v-card-text>
            </v-card>
        </div>
        <v-list v-else lines="two" data-testid="upcoming-list">
            <v-list-item
                v-for="row in rows"
                :key="`${row.kind}-${row.person_id ?? row.partnership_id ?? row.next_date}`"
                :data-testid="`upcoming-row-${row.kind}`"
                :prepend-icon="iconFor[row.kind] ?? 'calendar-clock'"
                @click="onRowClick(row)"
            >
                <v-list-item-title>{{ row.label }}</v-list-item-title>
                <v-list-item-subtitle>
                    {{ formatDate(row.next_date) }} · {{ relative(row.next_date) }}
                </v-list-item-subtitle>
                <template #append>
                    <v-chip size="small" variant="tonal" :data-testid="`upcoming-kind-${row.kind}`">
                        {{ t(`upcoming.kinds.${row.kind}`) }}
                    </v-chip>
                </template>
            </v-list-item>
        </v-list>
    </div>
</template>

<style scoped>
.upcoming-page {
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

.empty-state {
    display: flex;
    justify-content: center;
    padding: 2rem 0;
}

.filter-buttons {
    display: inline-flex;
    gap: 0.5rem;
}
</style>
