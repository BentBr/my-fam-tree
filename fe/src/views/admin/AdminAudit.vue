<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { type AuditFilter, useAuditList } from '@/api/hooks/audit'

const { t, locale } = useI18n()
const router = useRouter()

// Page-size dropdown is fixed to the four values the BE accepts; pushing
// anything else falls back to 50 server-side, which would silently
// desync the FE paginator's `length` computation. Keep the union tight.
const PAGE_SIZES = [50, 100, 200, 500] as const
type PageSize = (typeof PAGE_SIZES)[number]

// Pre-populated dropdown options. New audit `(action, entity_kind)`
// pairs introduced by Phases D / E (invite, owner_transfer) are wired
// here; the i18n catalog already carries the matching keys.
const ACTION_OPTIONS = [
    'create',
    'update',
    'delete',
    'invite',
    'accept_invite',
    'verify',
    'cancel',
    'set_role',
    'remove',
    'begin',
    'confirm',
    'complete',
] as const

const ENTITY_KIND_OPTIONS = [
    'person',
    'contact',
    'parent_link',
    'partnership',
    'family',
    'membership',
    'invite',
    'owner_transfer',
] as const

const page = ref<number>(1)
const pageSize = ref<PageSize>(50)
const action = ref<string | null>(null)
const entityKind = ref<string | null>(null)
const fromDate = ref<string | null>(null)
const toDate = ref<string | null>(null)

const filter = computed<AuditFilter>(() => {
    const f: AuditFilter = { page: page.value, pageSize: pageSize.value }
    if (action.value !== null) f.action = action.value
    if (entityKind.value !== null) f.entityKind = entityKind.value
    // Lift the calendar inputs to inclusive day boundaries in UTC. The
    // BE clamps with `>=` and `<=`, so widening to 00:00 / 23:59:59
    // makes the half-open shape match user intent ("between Mon and
    // Wed" includes all of Wed).
    if (fromDate.value !== null) f.from = `${fromDate.value}T00:00:00Z`
    if (toDate.value !== null) f.to = `${toDate.value}T23:59:59Z`
    return f
})

const query = useAuditList(filter)

// Reset to page 1 when any non-page filter changes — otherwise toggling
// a filter on page 3 silently keeps the offset and the user sees the
// empty middle of the new result set.
watch([action, entityKind, fromDate, toDate, pageSize], () => {
    page.value = 1
})

const rows = computed(() => query.data.value?.data ?? [])
const total = computed(() => query.data.value?.total ?? 0)
const pageCount = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value)))

const dateFmt = computed(
    () =>
        new Intl.DateTimeFormat(locale.value, {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        }),
)

function fmtDate(iso: string): string {
    return dateFmt.value.format(new Date(iso))
}

function clearFilters(): void {
    action.value = null
    entityKind.value = null
    fromDate.value = null
    toDate.value = null
    page.value = 1
}

function openEntity(personId: string | null | undefined): void {
    if (personId === null || personId === undefined || personId === '') return
    void router.push({ path: '/tree', query: { center: personId } })
}

// vue-i18n returns the key itself when a translation is missing, which is a
// safe fallback for unknown BE slugs (new actions / entity kinds introduced
// by future phases will surface as the raw slug until the catalog catches
// up). No `te()` pre-check needed.
function actionLabel(slug: string): string {
    return t(`admin.audit.action.${slug}`)
}

function entityKindLabel(slug: string): string {
    return t(`admin.audit.entityKind.${slug}`)
}

/**
 * Resolve the actor column to a single display string. The BE returns
 * `actor_display_name` (account `users.display_name`), `actor_person_name`
 * (linked-person name in this family, if any), and `actor_email`, plus
 * a nullable `actor_user_id` (system actions have no actor). The chain:
 *   display_name → linked-person name → email → '—'
 * Empty strings count as missing (the BE sends `display_name: ''` when
 * the user never set one rather than null). System rows (no actor) end
 * up at '—'.
 */
function actorLabel(row: {
    actor_user_id?: string | null
    actor_display_name?: string | null
    actor_person_name?: string | null
    actor_email?: string | null
}): string {
    if ((row.actor_user_id ?? '') === '') return '—'
    const dn = (row.actor_display_name ?? '').trim()
    if (dn !== '') return dn
    const pn = (row.actor_person_name ?? '').trim()
    if (pn !== '') return pn
    const em = (row.actor_email ?? '').trim()
    if (em !== '') return em
    return '—'
}

// Pull the invitee email + role out of the audit row's metadata blob so
// `(invite, membership)` rows can render a "Invited {email} as {role}"
// secondary line. The BE writes both fields (families::invite) but
// metadata is `serde_json::Value` so we defensively coerce.
interface InviteMetadata {
    email?: string
    role?: string
}

function inviteMetadata(metadata: unknown): InviteMetadata | null {
    if (metadata === null || typeof metadata !== 'object') return null
    const m = metadata as Record<string, unknown>
    const email = typeof m['email'] === 'string' ? m['email'] : undefined
    const role = typeof m['role'] === 'string' ? m['role'] : undefined
    if (email === undefined && role === undefined) return null
    const out: InviteMetadata = {}
    if (email !== undefined) out.email = email
    if (role !== undefined) out.role = role
    return out
}
</script>

<template>
    <section class="audit-page" data-testid="admin-audit-page">
        <header class="d-flex align-center mb-3">
            <h2 class="text-h6">{{ t('admin.audit.title') }}</h2>
        </header>

        <div class="filters mb-3" data-testid="admin-audit-filters">
            <v-text-field
                v-model="fromDate"
                :label="t('admin.audit.filter.from')"
                type="date"
                density="compact"
                hide-details
                clearable
                data-testid="admin-audit-filter-from"
            />
            <v-text-field
                v-model="toDate"
                :label="t('admin.audit.filter.to')"
                type="date"
                density="compact"
                hide-details
                clearable
                data-testid="admin-audit-filter-to"
            />
            <v-select
                v-model="action"
                :label="t('admin.audit.filter.action')"
                :items="ACTION_OPTIONS.map((a) => ({ title: actionLabel(a), value: a }))"
                density="compact"
                hide-details
                clearable
                data-testid="admin-audit-filter-action"
            />
            <v-select
                v-model="entityKind"
                :label="t('admin.audit.filter.entityKind')"
                :items="ENTITY_KIND_OPTIONS.map((k) => ({ title: entityKindLabel(k), value: k }))"
                density="compact"
                hide-details
                clearable
                data-testid="admin-audit-filter-kind"
            />
            <v-btn variant="text" data-testid="admin-audit-filter-clear" @click="clearFilters">
                {{ t('admin.audit.filter.clear') }}
            </v-btn>
        </div>

        <v-skeleton-loader v-if="query.isLoading.value" type="table" />
        <v-alert v-else-if="query.error.value !== null" type="error">{{ t('errors.generic') }}</v-alert>
        <v-card v-else variant="outlined">
            <v-table density="compact" data-testid="admin-audit-table">
                <thead>
                    <tr>
                        <th>{{ t('admin.audit.col.when') }}</th>
                        <th>{{ t('admin.audit.col.actor') }}</th>
                        <th>{{ t('admin.audit.col.action') }}</th>
                        <th>{{ t('admin.audit.col.entity') }}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-if="rows.length === 0" data-testid="admin-audit-empty">
                        <td colspan="4" class="text-medium-emphasis">
                            {{ t('admin.audit.empty') }}
                        </td>
                    </tr>
                    <tr v-for="row in rows" :key="row.id" :data-testid="`admin-audit-row-${row.id}`">
                        <td>{{ fmtDate(row.created_at) }}</td>
                        <td>{{ actorLabel(row) }}</td>
                        <td>
                            {{ actionLabel(row.action) }}
                            <span class="text-medium-emphasis"> · {{ entityKindLabel(row.entity_kind) }} </span>
                        </td>
                        <td>
                            <a
                                v-if="row.entity_person_id !== null && row.entity_person_id !== undefined"
                                class="entity-link"
                                role="link"
                                :data-testid="`admin-audit-entity-${row.id}`"
                                @click.prevent="openEntity(row.entity_person_id)"
                            >
                                {{ row.entity_person_name ?? row.entity_person_id }}
                            </a>
                            <span v-else class="text-medium-emphasis">
                                {{ entityKindLabel(row.entity_kind) }}
                            </span>
                            <!-- For (invite, membership) rows the row already
                                 shows the linked person via entity_person_id;
                                 the secondary line surfaces the invitee email
                                 + role from metadata so admins can see who was
                                 invited to be without opening the row. -->
                            <div
                                v-if="row.action === 'invite' && inviteMetadata(row.metadata) !== null"
                                class="text-caption text-medium-emphasis"
                                :data-testid="`admin-audit-invite-details-${row.id}`"
                            >
                                {{
                                    t('admin.audit.inviteDetails', {
                                        email: inviteMetadata(row.metadata)?.email ?? '—',
                                        role: inviteMetadata(row.metadata)?.role ?? '—',
                                    })
                                }}
                            </div>
                        </td>
                    </tr>
                </tbody>
            </v-table>
        </v-card>

        <footer class="pagination mt-3 d-flex align-center justify-end ga-3">
            <span class="text-medium-emphasis">{{ t('admin.audit.rowsPerPage') }}</span>
            <v-select
                v-model="pageSize"
                :items="[...PAGE_SIZES]"
                density="compact"
                hide-details
                style="max-width: 90px"
                data-testid="admin-audit-page-size"
            />
            <v-pagination v-model="page" :length="pageCount" :total-visible="5" data-testid="admin-audit-paginator" />
        </footer>
    </section>
</template>

<style scoped>
.filters {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr)) auto;
    gap: 0.75rem;
    align-items: end;
}
.entity-link {
    color: rgb(var(--v-theme-primary));
    cursor: pointer;
    text-decoration: none;
}
.entity-link:hover {
    text-decoration: underline;
}
</style>
