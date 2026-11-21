<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { useCreateInvite } from '@/api/hooks/invites'
import type { PersonView } from '@/api/types'

const props = defineProps<{
    person: PersonView
}>()

const open = defineModel<boolean>({ required: true })

const { t } = useI18n()

const inviteMutation = useCreateInvite()
const inviteEmail = ref('')
const inviteRole = ref<'user' | 'admin'>('user')

// Every (re-)open starts from a blank form.
watch(open, (isOpen) => {
    if (!isOpen) return
    inviteEmail.value = ''
    inviteRole.value = 'user'
})

async function submitInvite(): Promise<void> {
    await inviteMutation.mutateAsync({
        email: inviteEmail.value.trim(),
        role: inviteRole.value,
        personId: props.person.id,
    })
    open.value = false
}
</script>

<template>
    <v-dialog v-model="open" max-width="480">
        <v-card data-testid="person-invite-modal">
            <v-card-title>
                {{
                    t('person.invite.modalTitle', {
                        name: `${person.given_name} ${person.family_name}`.trim(),
                    })
                }}
            </v-card-title>
            <v-card-text class="invite-form">
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
                <v-btn variant="text" @click="open = false">
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
</template>

<style scoped>
/* Invite modal field-gap — same 12 px the PersonEdit / Account /
   Relations sub-forms use. Without it, Vuetify's floated label on a
   focused field (e.g., "Rolle" while the role picker is active) clips
   into the field above ("E-Mail-Adresse"). */
.invite-form > .v-input,
.invite-form > .v-text-field,
.invite-form > .v-select {
    margin-bottom: 12px;
}
</style>
