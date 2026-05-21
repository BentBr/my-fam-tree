<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { useTree } from '@/api/hooks/relationships'
import FamilyTree from '@/components/tree/FamilyTree.vue'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'

import PersonDetail from './PersonDetail.vue'
import PersonEdit from './PersonEdit.vue'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const tree = useTree()
const auth = useAuthStore()
const family = useActiveFamilyStore()

const selectedId = ref<string | null>(null)
const creating = ref(false)

/**
 * Center-on-member resolution order:
 *   1. ?center=<personId> query param (one-shot deep link).
 *   2. useActiveFamilyStore.focusedPersonId (persisted choice).
 *   3. The TreeNode whose `linked_user_id` equals the signed-in user.id.
 *   4. null — FamilyTree falls back to the layout origin + 40px gutter.
 */
const centerOnId = computed<string | null>(() => {
    const fromQuery = typeof route.query['center'] === 'string' ? route.query['center'] : null
    if (fromQuery !== null && fromQuery !== '') return fromQuery
    if (family.focusedPersonId !== null) return family.focusedPersonId
    const userId = auth.user?.id ?? null
    if (userId === null) return null
    const me = tree.data.value?.nodes.find(
        (n) => n.linked_user_id !== null && n.linked_user_id !== undefined && n.linked_user_id === userId,
    )
    return me?.id ?? null
})

/** Persist the new focal point so the next visit lands on the same node. */
function onSelect(id: string): void {
    selectedId.value = id
    family.setFocusedPerson(id)
}

function closeDrawer(): void {
    selectedId.value = null
    creating.value = false
}

function onDrawerUpdate(open: boolean): void {
    if (!open) closeDrawer()
}

function onCreateClick(): void {
    creating.value = true
    selectedId.value = null
}

function onChanged(): void {
    void tree.refetch()
}

function onCreated(id: string): void {
    creating.value = false
    onSelect(id)
    void tree.refetch()
}

// One-shot URL param: after the initial centerOnId is resolved we drop the
// `?center=…` query string so a hard reload doesn't fight against a
// subsequently-persisted focusedPersonId.
watch(
    () => route.query['center'],
    (val) => {
        if (typeof val === 'string' && val !== '') {
            const rest = { ...route.query }
            delete rest['center']
            void router.replace({ query: rest })
        }
    },
    { immediate: true },
)
</script>

<template>
    <div class="tree-page">
        <v-toolbar density="comfortable" elevation="0" color="transparent">
            <v-toolbar-title>{{ t('tree.title') }}</v-toolbar-title>
            <v-spacer />
            <v-btn prepend-icon="user-plus" color="primary" data-testid="tree-add-person" @click="onCreateClick">
                {{ t('tree.addPerson') }}
            </v-btn>
        </v-toolbar>

        <div class="tree-row">
            <div class="canvas">
                <v-skeleton-loader v-if="tree.isLoading.value" type="image" />
                <v-alert v-else-if="tree.error.value" type="error" data-testid="tree-error">
                    {{ t('tree.error') }}
                </v-alert>
                <FamilyTree
                    v-else-if="tree.data.value"
                    :tree="tree.data.value"
                    :selected-id="selectedId"
                    :center-on-id="centerOnId"
                    @select="onSelect"
                />
            </div>

            <v-navigation-drawer
                :model-value="selectedId !== null || creating"
                location="right"
                :width="380"
                temporary
                data-testid="person-drawer"
                @update:model-value="onDrawerUpdate"
            >
                <PersonDetail
                    v-if="selectedId !== null"
                    :person-id="selectedId"
                    @close="closeDrawer"
                    @changed="onChanged"
                />
                <div v-else-if="creating" class="pa-4">
                    <h3 class="text-h6 mb-3">{{ t('tree.addPerson') }}</h3>
                    <PersonEdit mode="create" @saved="onCreated" @cancel="closeDrawer" />
                </div>
            </v-navigation-drawer>
        </div>
    </div>
</template>

<style scoped>
.tree-page {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}
.tree-row {
    display: flex;
    flex: 1;
    min-height: 0;
}
.canvas {
    flex: 1;
    min-width: 0;
}
</style>
