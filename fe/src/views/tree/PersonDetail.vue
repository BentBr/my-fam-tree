<script setup lang="ts">
import { computed, ref, toRef } from 'vue'
import { useI18n } from 'vue-i18n'

import { useCreateInvite } from '@/api/hooks/invites'
import {
    useClaimPerson,
    useClearPersonPhoto,
    useDeletePerson,
    useGetPerson,
    useListPersons,
    useSetFavourite,
    useSetPersonPhoto,
} from '@/api/hooks/persons'
import DefaultAvatar from '@/components/common/DefaultAvatar.vue'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'

import ContactsSection from './ContactsSection.vue'
import PersonEdit from './PersonEdit.vue'
import PersonRelations from './PersonRelations.vue'

const props = defineProps<{
    personId: string
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'changed'): void
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
const inviteMutation = useCreateInvite()
const inviteOpen = ref(false)
const inviteEmail = ref('')
const inviteRole = ref<'user' | 'admin'>('user')

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

function openInvite(): void {
    inviteEmail.value = ''
    inviteRole.value = 'user'
    inviteOpen.value = true
}

async function submitInvite(): Promise<void> {
    if (person.value === null) return
    await inviteMutation.mutateAsync({
        email: inviteEmail.value.trim(),
        role: inviteRole.value,
        personId: person.value.id,
    })
    inviteOpen.value = false
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

// ---------------------------------------------------------------------------
// Photo upload
// ---------------------------------------------------------------------------
//
// The hidden file input is triggered programmatically by the avatar button,
// so the only UI surface is the avatar itself + a clear-photo affordance.
// `canEdit` already gates whether the user is allowed to touch this person;
// the BE re-checks anyway (defense in depth).
const setPhoto = useSetPersonPhoto()
const clearPhoto = useClearPersonPhoto()
const fileInput = ref<HTMLInputElement | null>(null)
const photoBusy = computed(() => setPhoto.isPending.value || clearPhoto.isPending.value)
const displayName = computed(() => {
    if (person.value === null) return ''
    const first = person.value.given_name ?? ''
    const last = person.value.family_name ?? ''
    return `${first} ${last}`.trim()
})

function openPhotoPicker(): void {
    fileInput.value?.click()
}

async function onPhotoSelected(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement
    const file = input.files?.[0] ?? null
    // Reset the input value BEFORE the await so re-selecting the same file
    // still fires a change event (browsers de-dupe by filename otherwise).
    input.value = ''
    if (file === null || person.value === null) return
    await setPhoto.mutateAsync({ id: person.value.id, file })
}

async function removePhoto(): Promise<void> {
    if (person.value === null) return
    await clearPhoto.mutateAsync(person.value.id)
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
        <!--
            Big squared hero photo. Full sidebar width, 1:1 aspect ratio,
            corners only slightly rounded so it reads as a card rather
            than an avatar (the rest of the UI uses round avatars).
            Click-to-upload affordance overlays the bottom-right.
        -->
        <div class="person-detail-hero mb-3" data-testid="person-detail-hero">
            <img
                v-if="person?.photo_url"
                :src="person.photo_url"
                :alt="displayName"
                class="person-detail-hero-img"
                data-testid="person-detail-hero-photo"
            />
            <div v-else class="person-detail-hero-fallback" data-testid="person-detail-hero-fallback">
                <DefaultAvatar :src="null" :name="displayName" :size="180" />
            </div>
            <v-btn
                v-if="canEdit && person !== null"
                icon="mdi-camera"
                size="small"
                color="primary"
                class="person-detail-hero-edit"
                :loading="photoBusy"
                :aria-label="t('person.photo.upload')"
                data-testid="person-detail-photo-upload"
                @click="openPhotoPicker"
            />
            <v-btn
                v-if="canEdit && person?.photo_url"
                variant="flat"
                size="x-small"
                color="error"
                class="person-detail-hero-remove"
                :loading="photoBusy"
                :aria-label="t('person.photo.remove')"
                data-testid="person-detail-photo-remove"
                @click="removePhoto"
            >
                {{ t('person.photo.remove') }}
            </v-btn>
            <input
                ref="fileInput"
                type="file"
                accept="image/jpeg,image/png,image/webp"
                class="d-none"
                data-testid="person-detail-photo-input"
                @change="onPhotoSelected"
            />
        </div>

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

            <!-- Profile section — every PersonView field rendered as a labelled
                 read-only row. Editing flips into the existing PersonEdit
                 component (covers every field via v-text-field/v-combobox);
                 we deliberately don't duplicate that form here. -->
            <v-list density="compact" class="mb-2" data-testid="person-profile-list">
                <v-list-item data-testid="person-field-given-name">
                    <v-list-item-title>{{ t('person.fields.given_name') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.given_name }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-family-name">
                    <v-list-item-title>{{ t('person.fields.family_name') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.family_name || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-name-at-birth">
                    <v-list-item-title>{{ t('person.fields.name_at_birth') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.name_at_birth || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-nickname">
                    <v-list-item-title>{{ t('person.fields.nickname') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.nickname || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-gender">
                    <v-list-item-title>{{ t('person.fields.gender') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.gender || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-birth-date">
                    <v-list-item-title>{{ t('person.fields.birth_date') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.birth_date ?? '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-birth-place">
                    <v-list-item-title>{{ t('person.fields.birth_place') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.birth_place || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-death-date">
                    <v-list-item-title>{{ t('person.fields.death_date') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.death_date ?? '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item v-if="person.notes" data-testid="person-field-notes">
                    <v-list-item-title>{{ t('person.fields.notes') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.notes }}</v-list-item-subtitle>
                </v-list-item>
            </v-list>

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

            <v-dialog v-model="inviteOpen" max-width="480">
                <v-card v-if="person !== null" data-testid="person-invite-modal">
                    <v-card-title>
                        {{
                            t('person.invite.modalTitle', {
                                name: `${person.given_name} ${person.family_name}`.trim(),
                            })
                        }}
                    </v-card-title>
                    <v-card-text>
                        <v-text-field
                            v-model="inviteEmail"
                            :label="t('person.invite.emailLabel')"
                            type="email"
                            density="compact"
                            data-testid="person-invite-email"
                        />
                        <v-select
                            v-model="inviteRole"
                            :label="t('person.invite.roleLabel')"
                            :items="[
                                { title: t('admin.members.role.user'), value: 'user' },
                                { title: t('admin.members.role.admin'), value: 'admin' },
                            ]"
                            density="compact"
                            hide-details
                            data-testid="person-invite-role"
                        />
                    </v-card-text>
                    <v-card-actions>
                        <v-spacer />
                        <v-btn variant="text" @click="inviteOpen = false">
                            {{ t('common.cancel') }}
                        </v-btn>
                        <v-btn
                            color="primary"
                            variant="flat"
                            :disabled="inviteEmail.trim().length === 0"
                            :loading="inviteMutation.isPending.value"
                            data-testid="person-invite-submit"
                            @click="submitInvite"
                        >
                            {{ t('person.invite.submit') }}
                        </v-btn>
                    </v-card-actions>
                </v-card>
            </v-dialog>

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

<style scoped>
/* Squared, full-sidebar-width hero photo. The card is 1:1 so the
   aspect ratio matches social-style profile photos; only the corners
   are slightly rounded so it reads as "card" rather than "avatar"
   (the rest of the UI uses round avatars). */
.person-detail-hero {
    position: relative;
    width: 100%;
    aspect-ratio: 1 / 1;
    border-radius: 8px;
    overflow: hidden;
    background: rgb(var(--v-theme-surface-variant, 240 240 240));
}

.person-detail-hero-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
}

/* When there's no photo the DefaultAvatar (with initials) is centred
   in the same square so the layout doesn't jump when a photo lands. */
.person-detail-hero-fallback {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
}

/* Camera button overlays the bottom-right of the hero photo — the
   conventional "edit my photo" affordance on every social platform. */
.person-detail-hero-edit {
    position: absolute;
    bottom: 8px;
    right: 8px;
}

/* "Remove photo" is a destructive action; tuck it bottom-left away
   from the primary affordance so it can't be hit accidentally. */
.person-detail-hero-remove {
    position: absolute;
    bottom: 8px;
    left: 8px;
}
</style>
