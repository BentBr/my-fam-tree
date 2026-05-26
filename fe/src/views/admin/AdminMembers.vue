<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { type MemberRow, useMembers, useRevokeMember, useSetRole } from '@/api/hooks/members'
import { useBeginOwnerTransfer, useCancelOwnerTransfer, useOwnerTransfer } from '@/api/hooks/owner_transfer'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'

const { t, locale } = useI18n()
const family = useActiveFamilyStore()
const auth = useAuthStore()
const q = useMembers()
const setRole = useSetRole()
const revoke = useRevokeMember()
const transferQ = useOwnerTransfer()
const beginTransfer = useBeginOwnerTransfer()
const cancelTransfer = useCancelOwnerTransfer()

const rows = computed<MemberRow[]>(() => q.data.value ?? [])
const myUserId = computed<string | null>(() => auth.user?.id ?? null)
const myRole = computed(() => family.activeFamily?.role ?? null)

const pendingTransfer = computed(() => transferQ.data.value ?? null)

function canTransferTo(r: MemberRow): boolean {
    if (myRole.value !== 'owner') return false
    if (r.role !== 'admin') return false
    return r.user_id !== myUserId.value
}

function nameOf(userId: string): string {
    return rows.value.find((r) => r.user_id === userId)?.display_name ?? userId
}

const transferTarget = ref<MemberRow | null>(null)
const transferDialogOpen = computed<boolean>({
    get: () => transferTarget.value !== null,
    set: (v) => {
        if (!v) transferTarget.value = null
    },
})

function openTransferDialog(r: MemberRow): void {
    transferTarget.value = r
}

async function submitTransfer(): Promise<void> {
    const target = transferTarget.value
    if (target === null) return
    await beginTransfer.mutateAsync(target.user_id)
    transferTarget.value = null
}

// Role matrix gates. The BE is the source of truth and will reject any
// disallowed mutation; these checks only decide which buttons render.
function canPromote(r: MemberRow): boolean {
    if (r.user_id === myUserId.value) return false
    if (r.role !== 'user') return false
    return myRole.value === 'admin' || myRole.value === 'owner'
}

function canDemote(r: MemberRow): boolean {
    if (r.user_id === myUserId.value) return false
    if (r.role !== 'admin') return false
    return myRole.value === 'owner'
}

function canRevoke(r: MemberRow): boolean {
    if (r.user_id === myUserId.value) return false
    if (r.role === 'owner') return false
    if (myRole.value === 'owner') return true
    if (myRole.value === 'admin') return r.role === 'user'
    return false
}

type ConfirmKind = 'revoke' | 'demote'
interface ConfirmTarget {
    kind: ConfirmKind
    row: MemberRow
}
const confirmTarget = ref<ConfirmTarget | null>(null)
const dialogOpen = computed<boolean>({
    get: () => confirmTarget.value !== null,
    set: (v) => {
        if (!v) confirmTarget.value = null
    },
})

function doPromote(r: MemberRow): void {
    setRole.mutate({ userId: r.user_id, role: 'admin' })
}

function confirmAction(): void {
    const target = confirmTarget.value
    if (target === null) return
    if (target.kind === 'revoke') {
        revoke.mutate(target.row.user_id)
    } else {
        setRole.mutate({ userId: target.row.user_id, role: 'user' })
    }
    confirmTarget.value = null
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

function roleColor(role: MemberRow['role']): string | undefined {
    if (role === 'owner') return 'primary'
    if (role === 'admin') return 'info'
    return undefined
}
</script>

<template>
    <section class="members-page" data-testid="admin-members-page">
        <header class="d-flex align-center mb-3">
            <h2 class="text-h6">{{ t('admin.members.title') }}</h2>
        </header>

        <v-alert
            v-if="pendingTransfer !== null"
            type="info"
            variant="tonal"
            class="mb-3"
            data-testid="admin-members-transfer-banner"
        >
            <div class="d-flex align-center" style="gap: 8px">
                <div class="flex-grow-1">
                    {{
                        t('admin.transfer.pendingBanner', {
                            name: nameOf(pendingTransfer.to_user_id),
                            from: pendingTransfer.from_confirmed
                                ? t('admin.transfer.stateConfirmed')
                                : t('admin.transfer.stateWaiting'),
                            to: pendingTransfer.to_confirmed
                                ? t('admin.transfer.stateConfirmed')
                                : t('admin.transfer.stateWaiting'),
                        })
                    }}
                </div>
                <v-btn
                    v-if="myRole === 'owner'"
                    variant="text"
                    size="small"
                    color="error"
                    data-testid="admin-members-transfer-cancel"
                    @click="cancelTransfer.mutate()"
                >
                    {{ t('admin.transfer.cancel') }}
                </v-btn>
            </div>
        </v-alert>

        <v-skeleton-loader v-if="q.isLoading.value" type="table" />
        <v-alert v-else-if="q.error.value !== null" type="error">{{ t('errors.generic') }}</v-alert>
        <v-card v-else variant="outlined">
            <v-table density="compact" data-testid="admin-members-table">
                <thead>
                    <tr>
                        <th>{{ t('admin.members.col.name') }}</th>
                        <th>{{ t('admin.members.col.email') }}</th>
                        <th>{{ t('admin.members.col.role') }}</th>
                        <th>{{ t('admin.members.col.joined') }}</th>
                        <th>{{ t('admin.members.col.actions') }}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-if="rows.length === 0" data-testid="admin-members-empty">
                        <td colspan="5" class="text-medium-emphasis">
                            {{ t('admin.members.empty') }}
                        </td>
                    </tr>
                    <tr v-for="r in rows" :key="r.user_id" :data-testid="`admin-members-row-${r.user_id}`">
                        <td>{{ r.display_name }}</td>
                        <td>{{ r.email }}</td>
                        <td>
                            <v-chip size="small" :color="roleColor(r.role)">
                                {{ t(`admin.members.role.${r.role}`) }}
                            </v-chip>
                        </td>
                        <td>{{ fmtDate(r.joined_at) }}</td>
                        <td>
                            <v-btn
                                v-if="canPromote(r)"
                                size="small"
                                variant="text"
                                :data-testid="`admin-members-promote-${r.user_id}`"
                                @click="doPromote(r)"
                            >
                                {{ t('admin.members.action.promote') }}
                            </v-btn>
                            <v-btn
                                v-if="canDemote(r)"
                                size="small"
                                variant="text"
                                :data-testid="`admin-members-demote-${r.user_id}`"
                                @click="confirmTarget = { kind: 'demote', row: r }"
                            >
                                {{ t('admin.members.action.demote') }}
                            </v-btn>
                            <v-btn
                                v-if="canRevoke(r)"
                                size="small"
                                variant="text"
                                color="error"
                                :data-testid="`admin-members-revoke-${r.user_id}`"
                                @click="confirmTarget = { kind: 'revoke', row: r }"
                            >
                                {{ t('admin.members.action.revoke') }}
                            </v-btn>
                            <v-btn
                                v-if="canTransferTo(r) && pendingTransfer === null"
                                size="small"
                                variant="text"
                                :data-testid="`admin-members-transfer-${r.user_id}`"
                                @click="openTransferDialog(r)"
                            >
                                {{ t('admin.transfer.cta') }}
                            </v-btn>
                        </td>
                    </tr>
                </tbody>
            </v-table>
        </v-card>

        <v-dialog v-model="transferDialogOpen" max-width="480">
            <v-card v-if="transferTarget !== null" data-testid="admin-members-transfer-dialog">
                <v-card-title>
                    {{ t('admin.transfer.modalTitle', { name: transferTarget.display_name }) }}
                </v-card-title>
                <v-card-text>
                    {{ t('admin.transfer.modalText', { name: transferTarget.display_name }) }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="transferTarget = null">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn
                        color="primary"
                        variant="flat"
                        data-testid="admin-members-transfer-submit"
                        @click="submitTransfer"
                    >
                        {{ t('admin.transfer.submit') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <v-dialog v-model="dialogOpen" max-width="420">
            <v-card v-if="confirmTarget !== null" data-testid="admin-members-confirm-dialog">
                <v-card-title>
                    {{
                        confirmTarget.kind === 'revoke'
                            ? t('admin.members.confirm.revokeTitle')
                            : t('admin.members.confirm.demoteTitle', {
                                  name: confirmTarget.row.display_name,
                              })
                    }}
                </v-card-title>
                <v-card-text>
                    {{
                        confirmTarget.kind === 'revoke'
                            ? t('admin.members.confirm.revokeText', {
                                  name: confirmTarget.row.display_name,
                              })
                            : t('admin.members.confirm.demoteText', {
                                  name: confirmTarget.row.display_name,
                              })
                    }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="confirmTarget = null">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn color="error" variant="flat" data-testid="admin-members-confirm" @click="confirmAction">
                        {{ t('common.delete') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </section>
</template>

<style scoped>
.members-page {
    width: 100%;
}
</style>
