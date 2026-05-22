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

const query = useUpcoming(filter)

const iconFor: Record<string, string> = {
    birthday: 'cake',
    wedding_anniversary: 'heart',
    death_anniversary: 'candle',
}

const dateFormatter = computed(() =>
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
    if (row.person_id !== null && row.person_id !== undefined && row.person_id !== '') {
        void router.push({ path: '/tree', query: { center: row.person_id } })
    }
    // Wedding-anniversary rows have no person_id; clicking is a no-op
    // for now. A future iteration could route via partnership_id once
    // /tree understands centering on a partnership edge.
}
</script>

<template>
    <div class="upcoming-page" data-testid="upcoming-page">
        <v-toolbar density="comfortable" elevation="0" color="transparent">
            <v-toolbar-title>{{ t('upcoming.title') }}</v-toolbar-title>
            <v-spacer />
            <v-btn-toggle
                v-model="filter"
                color="primary"
                rounded="lg"
                density="comfortable"
                mandatory="force"
                divided
                data-testid="upcoming-filter"
            >
                <v-btn
                    value="birthday"
                    prepend-icon="cake"
                    data-testid="upcoming-filter-birthday"
                    @click.stop="toggle('birthday')"
                >
                    {{ t('upcoming.filter.birthday') }}
                </v-btn>
                <v-btn
                    value="anniversary"
                    prepend-icon="heart"
                    data-testid="upcoming-filter-anniversary"
                    @click.stop="toggle('anniversary')"
                >
                    {{ t('upcoming.filter.anniversary') }}
                </v-btn>
            </v-btn-toggle>
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
</style>
