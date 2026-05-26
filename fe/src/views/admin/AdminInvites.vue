<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { type InviteRow, useCancelInvite, useInvites } from '@/api/hooks/invites'

const { t, locale } = useI18n()
const router = useRouter()
const q = useInvites()
const cancel = useCancelInvite()

const rows = computed<InviteRow[]>(() => q.data.value ?? [])

const confirmId = ref<string | null>(null)
const confirmOpen = computed<boolean>({
    get: () => confirmId.value !== null,
    set: (v) => {
        if (!v) confirmId.value = null
    },
})

function askCancel(id: string): void {
    confirmId.value = id
}

function doCancel(): void {
    const id = confirmId.value
    if (id === null) return
    cancel.mutate(id)
    confirmId.value = null
}

const dateFmt = computed(
    () =>
        new Intl.DateTimeFormat(locale.value, {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
        }),
)
function fmtDate(iso: string): string {
    return dateFmt.value.format(new Date(iso))
}

function openPerson(id: string | null): void {
    if (id === null) return
    void router.push({ path: '/tree', query: { center: id } })
}
</script>

<template>
    <section class="invites-page" data-testid="admin-invites-page">
        <header class="d-flex align-center mb-3">
            <h2 class="text-h6">{{ t('admin.invites.title') }}</h2>
        </header>

        <v-skeleton-loader v-if="q.isLoading.value" type="table" />
        <v-alert v-else-if="q.error.value !== null" type="error">{{ t('errors.generic') }}</v-alert>
        <v-card v-else variant="outlined">
            <v-table density="compact" data-testid="admin-invites-table">
                <thead>
                    <tr>
                        <th>{{ t('admin.invites.col.email') }}</th>
                        <th>{{ t('admin.invites.col.role') }}</th>
                        <th>{{ t('admin.invites.col.person') }}</th>
                        <th>{{ t('admin.invites.col.expires') }}</th>
                        <th>{{ t('admin.invites.col.actions') }}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-if="rows.length === 0" data-testid="admin-invites-empty">
                        <td colspan="5" class="text-medium-emphasis">
                            {{ t('admin.invites.empty') }}
                        </td>
                    </tr>
                    <tr v-for="r in rows" :key="r.id" :data-testid="`admin-invites-row-${r.id}`">
                        <td>{{ r.email }}</td>
                        <td>{{ t(`admin.members.role.${r.role}`) }}</td>
                        <td>
                            <a
                                v-if="r.person_id !== null"
                                class="entity-link"
                                :data-testid="`admin-invites-person-${r.id}`"
                                @click.prevent="openPerson(r.person_id)"
                            >
                                {{ r.person_id }}
                            </a>
                            <span v-else class="text-medium-emphasis">—</span>
                        </td>
                        <td>{{ fmtDate(r.expires_at) }}</td>
                        <td>
                            <v-btn
                                size="small"
                                variant="text"
                                color="error"
                                :data-testid="`admin-invites-cancel-${r.id}`"
                                @click="askCancel(r.id)"
                            >
                                {{ t('admin.invites.cancel') }}
                            </v-btn>
                        </td>
                    </tr>
                </tbody>
            </v-table>
        </v-card>

        <v-dialog v-model="confirmOpen" max-width="420">
            <v-card data-testid="admin-invites-confirm-dialog">
                <v-card-title>{{ t('admin.invites.confirm.cancelTitle') }}</v-card-title>
                <v-card-text>{{ t('admin.invites.confirm.cancelText') }}</v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="confirmId = null">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn color="error" variant="flat" data-testid="admin-invites-confirm" @click="doCancel">
                        {{ t('common.delete') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </section>
</template>

<style scoped>
.invites-page {
    width: 100%;
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
