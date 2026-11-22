<script setup lang="ts">
import { computed, ref, toRef } from 'vue'
import { useI18n } from 'vue-i18n'

import { useClaimPerson, useDeletePerson, useGetPerson, useListPersons, useSetFavourite } from '@/api/hooks/persons'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'

import ContactsSection from './ContactsSection.vue'
import PersonDetailHero from './PersonDetailHero.vue'
import PersonEdit from './PersonEdit.vue'
import PersonInviteDialog from './PersonInviteDialog.vue'
import PersonProfileList from './PersonProfileList.vue'
import PersonRelations from './PersonRelations.vue'

const props = defineProps<{
    personId: string
}>()

const emit = defineEmits<{
    close: []
    changed: []
}>()

const { t } = useI18n()

// `toRef(props, 'personId')` lets the useGetPerson query react to drawer
// navigation without remounting the component.
const personId = toRef(props, 'personId')

// Two data sources:
// - `useGetPerson` is the source of truth for the rendered profile fields —
//   per the brief, the drawer must always reflect the latest server state.
// - `useListPersons` is the fallback (also used by tests + while the GET is
//   in flight) so the header never flashes a blank name.
const personQ = useGetPerson(personId)
const list = useListPersons()
const del = useDeletePerson()

const family = useActiveFamilyStore()
const auth = useAuthStore()

const editing = ref(false)
const confirmDelete = ref(false)

const person = computed(() => personQ.data.value ?? list.data.value?.find((p) => p.id === props.personId) ?? null)

// owner+admin can edit any row. A `user`-role member can only edit the
// person row that maps to their own user (linked_user_id matches the
// signed-in user). `null` role happens during the brief window after
// sign-in before the active-family hydrates — render read-only then.
const canEdit = computed(() => {
    const role = family.activeFamily?.role ?? null
    if (role === 'owner' || role === 'admin') return true
    if (role === 'user' && person.value?.linked_user_id !== null && person.value?.linked_user_id !== undefined) {
        return person.value.linked_user_id === auth.user?.id
    }
    return false
})

// Invite-to-family modal. Visible to admin+owner only; gated on the
// active family's role (read from the same store as `canEdit` above so
// the two checks share their loading-window semantics).
const inviteOpen = ref(false)

const canInvite = computed(() => {
    const role = family.activeFamily?.role ?? null
    if (role !== 'admin' && role !== 'owner') return false
    // Hide the CTA once the person is already linked to a user account —
    // a second invite for the same row would either re-bind an unrelated
    // user (admins can do this if they really mean to, via the seam in
    // the FE update flow) or get rejected on accept because the
    // recipient's email already maps to a member.
    if (person.value?.linked_user_id !== null && person.value?.linked_user_id !== undefined) return false
    return true
})

// "Claim as me" — direct self-link, skipping the email round-trip the
// invite flow forces. Surface conditions:
//   - admin/owner (regular `user` members onboard via invite-email; the
//     consent round-trip is the gate. The shortcut is for the family
//     owner/admins who created a person row representing themselves);
//   - the person row isn't already linked (would be a no-op / 409);
//   - we know the caller's identity (auth.user.id present).
// We deliberately do NOT also check "caller isn't already linked to
// another person in this family" client-side — that requires walking
// the full persons list and the BE rejects it with a 409 + toast
// anyway, so the rare double-claim flow gets clear server feedback
// without bloating this computed.
const claim = useClaimPerson()
const canClaim = computed(() => {
    const role = family.activeFamily?.role ?? null
    if (role !== 'admin' && role !== 'owner') return false
    if (auth.user === null) return false
    if (person.value?.linked_user_id !== null && person.value?.linked_user_id !== undefined) return false
    return true
})

async function claimAsMe(): Promise<void> {
    if (person.value === null) return
    await claim.mutateAsync(person.value.id)
}

// The dialog resets its own form on every open (see PersonInviteDialog).
function openInvite(): void {
    inviteOpen.value = true
}

async function remove(): Promise<void> {
    await del.mutateAsync(props.personId)
    confirmDelete.value = false
    emit('changed')
    emit('close')
}

function onSaved(): void {
    editing.value = false
    emit('changed')
}

function onRelationsChanged(): void {
    emit('changed')
}

const fav = useSetFavourite()
const isFavourite = computed(() => person.value?.is_favourite_for_me === true)
function toggleFavourite(): void {
    if (person.value === null) return
    fav.mutate({ id: person.value.id, isFavourite: !isFavourite.value })
}

// The chip distinguishes "this person row is linked to *some* user
// account" from "this person row is linked to *YOU*". The latter gets a
// friendlier label and a different colour so the signed-in user
// immediately sees their own node in the tree drawer.
const isMe = computed(() => {
    const linked = person.value?.linked_user_id
    return linked !== null && linked !== undefined && linked === auth.user?.id
})
</script>

<template>
    <section class="pa-4" data-testid="person-detail">
        <PersonDetailHero :person="person" :can-edit="canEdit" />

        <header class="d-flex align-center justify-space-between mb-3">
            <div class="d-flex align-center ga-2 flex-wrap">
                <v-btn
                    v-if="person !== null"
                    variant="text"
                    size="small"
                    :color="isFavourite ? 'warning' : undefined"
                    :icon="isFavourite ? 'mdi-star' : 'mdi-star-outline'"
                    :aria-label="t(isFavourite ? 'person.favourite.unmarkTooltip' : 'person.favourite.markTooltip')"
                    :data-testid="`person-detail-favourite-${person.id}`"
                    @click="toggleFavourite"
                />
                <h3 v-if="person" class="text-h6" data-testid="person-detail-title">
                    {{ person.given_name }} {{ person.family_name }}
                </h3>
                <v-chip
                    v-if="isMe"
                    size="small"
                    color="success"
                    variant="tonal"
                    prepend-icon="user-check"
                    data-testid="person-its-you-chip"
                >
                    {{ t('person.itsYou') }}
                </v-chip>
                <v-chip
                    v-else-if="person?.linked_user_id"
                    size="small"
                    color="info"
                    variant="tonal"
                    prepend-icon="user-check"
                    data-testid="person-linked-account-chip"
                >
                    {{ t('person.linkedAccount') }}
                </v-chip>
            </div>
            <v-btn icon="x" variant="text" size="small" data-testid="person-detail-close" @click="emit('close')" />
        </header>

        <v-skeleton-loader v-if="personQ.isLoading.value && person === null" type="article" />

        <template v-else-if="!editing && person !== null">
            <div class="d-flex align-center justify-space-between mb-2">
                <h4 class="text-subtitle-1">{{ t('person.sections.profile') }}</h4>
                <v-chip
                    v-if="!canEdit"
                    size="small"
                    color="grey-lighten-1"
                    variant="tonal"
                    data-testid="person-readonly-badge"
                >
                    {{ t('common.readOnly') }}
                </v-chip>
            </div>

            <PersonProfileList :person="person" />

            <v-divider class="my-3" />

            <!-- Contacts section. Replaces the flat email/phone/address
                 fields with per-row entries from `person_contacts`. The
                 backend filters `admins_only` rows out for `user` role. -->
            <ContactsSection :person-id="props.personId" :linked-user-id="person.linked_user_id ?? null" />

            <v-divider class="my-3" />

            <PersonRelations :person-id="props.personId" :can-edit="canEdit" @changed="onRelationsChanged" />

            <v-divider class="my-3" />

            <div v-if="canEdit" class="d-flex ga-2 flex-wrap">
                <v-btn variant="outlined" data-testid="person-edit-button" @click="editing = true">
                    {{ t('common.edit') }}
                </v-btn>
                <v-btn
                    v-if="canClaim"
                    variant="outlined"
                    color="success"
                    prepend-icon="mdi-account-check"
                    :loading="claim.isPending.value"
                    data-testid="person-claim-cta"
                    @click="claimAsMe"
                >
                    {{ t('person.claim.cta') }}
                </v-btn>
                <v-btn
                    v-if="canInvite"
                    variant="outlined"
                    color="primary"
                    prepend-icon="mdi-account-plus"
                    data-testid="person-invite-cta"
                    @click="openInvite"
                >
                    {{ t('person.invite.cta') }}
                </v-btn>
                <v-btn
                    color="error"
                    variant="outlined"
                    data-testid="person-delete-button"
                    @click="confirmDelete = true"
                >
                    {{ t('common.delete') }}
                </v-btn>
            </div>

            <PersonInviteDialog v-model="inviteOpen" :person="person" />

            <v-dialog v-model="confirmDelete" max-width="420">
                <v-card>
                    <v-card-title>{{ t('person.confirm.deleteTitle') }}</v-card-title>
                    <v-card-text>{{ t('person.confirm.deleteText') }}</v-card-text>
                    <v-card-actions>
                        <v-spacer />
                        <v-btn variant="text" @click="confirmDelete = false">
                            {{ t('common.cancel') }}
                        </v-btn>
                        <v-btn
                            color="error"
                            :loading="del.isPending.value"
                            data-testid="person-delete-confirm"
                            @click="remove"
                        >
                            {{ t('common.delete') }}
                        </v-btn>
                    </v-card-actions>
                </v-card>
            </v-dialog>
        </template>

        <PersonEdit
            v-else-if="editing && person !== null"
            mode="edit"
            :initial="{
                id: person.id,
                given_name: person.given_name,
                family_name: person.family_name,
                name_at_birth: person.name_at_birth,
                nickname: person.nickname,
                gender: person.gender,
                birth_date: person.birth_date ?? null,
                birth_place: person.birth_place,
                death_date: person.death_date ?? null,
                notes: person.notes,
                linked_user_id: person.linked_user_id ?? null,
            }"
            @saved="onSaved"
            @cancel="editing = false"
        />
    </section>
</template>
