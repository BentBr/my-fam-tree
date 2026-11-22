<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { useClearPersonPhoto, useSetPersonPhoto } from '@/api/hooks/persons'
import type { PersonView } from '@/api/types'
import DefaultAvatar from '@/components/common/DefaultAvatar.vue'

const props = defineProps<{
    person: PersonView | null
    canEdit: boolean
}>()

const { t } = useI18n()

// The hidden file input is triggered programmatically by the avatar button,
// so the only UI surface is the avatar itself + a clear-photo affordance.
// `canEdit` already gates whether the user is allowed to touch this person;
// the BE re-checks anyway (defense in depth).
const setPhoto = useSetPersonPhoto()
const clearPhoto = useClearPersonPhoto()
const fileInput = ref<HTMLInputElement | null>(null)
const photoBusy = computed(() => setPhoto.isPending.value || clearPhoto.isPending.value)
const displayName = computed(() => {
    if (props.person === null) return ''
    return `${props.person.given_name} ${props.person.family_name}`.trim()
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
    if (file === null || props.person === null) return
    await setPhoto.mutateAsync({ id: props.person.id, file })
}

async function removePhoto(): Promise<void> {
    if (props.person === null) return
    await clearPhoto.mutateAsync(props.person.id)
}
</script>

<template>
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
    background: rgb(var(--v-theme-surface-variant));
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
