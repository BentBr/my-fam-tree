<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
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

const pageTitle = computed(() => {
    const name = family.activeFamily?.name
    if (name === undefined || name === '') return t('tree.title')
    return t('tree.titleOf', { name })
})

// `?center=<id>` is a one-shot deep link. We capture it ONCE at setup
// time into a local const so the URL-strip in onMounted can't yank the
// value out from under `centerOnId` before <FamilyTree> mounts. The
// previous implementation read `route.query.center` from inside the
// computed, so stripping the URL silently demoted the value to `null`
// and the drawer never opened.
const initialCenterParam: string | null =
    typeof route.query['center'] === 'string' && route.query['center'] !== '' ? route.query['center'] : null

// Vuetify's `v-navigation-drawer` (temporary variant) mounts with
// `isActive=false` when model-value is true at first paint — initial
// open is treated as a no-op so the user always sees a closed drawer
// on first load. We mirror the user-click pattern: mount with `null`
// (drawer mounts closed), then flip to the captured id in `onMounted`
// so Vuetify sees a `false → true` transition and opens.
const selectedId = ref<string | null>(null)
const creating = ref(false)

/**
 * Center-on-member resolution order:
 *   1. ?center=<personId> at mount time (one-shot deep link).
 *   2. useActiveFamilyStore.focusedPersonId (persisted choice).
 *   3. The TreeNode whose `linked_user_id` equals the signed-in user.id.
 *   4. null — FamilyTree falls back to the layout origin + 40px gutter.
 */
const centerOnId = computed<string | null>(() => {
    if (initialCenterParam !== null) return initialCenterParam
    if (family.focusedPersonId !== null) return family.focusedPersonId
    const userId = auth.user?.id ?? null
    if (userId === null) return null
    const me = tree.data.value?.nodes.find(
        (n) => n.linked_user_id !== null && n.linked_user_id !== undefined && n.linked_user_id === userId,
    )
    return me?.id ?? null
})

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

const treeRef = ref<{ refit: () => void } | null>(null)
function onFit(): void {
    treeRef.value?.refit()
}

function onCreated(id: string): void {
    creating.value = false
    onSelect(id)
    void tree.refetch()
}

// `v-navigation-drawer` (temporary variant) silently no-ops if it
// mounts with `model-value=true` — its internal `isActive` only tracks
// false → true transitions that happen *after* mount. We therefore:
//   1. Keep `selectedId` null at setup so the drawer mounts closed.
//   2. Flip it post-mount in a watcher gated on BOTH `isMounted` AND
//      `tree.data` being defined.
// `immediate: true` is intentionally NOT used — the cached case
// (second visit, tree query hot) would otherwise fire the watcher
// synchronously during setup, which puts us right back in the
// "drawer mounts already open" trap. Watching `isMounted` ensures
// the cached path still fires (the false → true edge from
// `onMounted` is its trigger), just safely post-paint.
const isMounted = ref(false)
onMounted(() => {
    isMounted.value = true
    if (initialCenterParam === null) return
    family.setFocusedPerson(initialCenterParam)
    const rest = { ...route.query }
    delete rest['center']
    void router.replace({ query: rest })
})

watch([isMounted, () => tree.data.value] as const, ([mounted, data]) => {
    if (initialCenterParam === null) return
    if (!mounted || data === undefined) return
    if (selectedId.value !== null) return
    selectedId.value = initialCenterParam
})
</script>

<template>
    <div class="tree-page">
        <v-toolbar density="comfortable" elevation="0" color="transparent">
            <v-toolbar-title data-testid="tree-page-title">{{ pageTitle }}</v-toolbar-title>
            <v-spacer />
            <v-btn
                v-if="tree.data.value !== undefined && tree.data.value.nodes.length > 0"
                prepend-icon="maximize"
                variant="text"
                data-testid="tree-fit-to-view"
                @click="onFit"
            >
                {{ t('tree.fitToView') }}
            </v-btn>
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
                <!-- Empty state: no persons in the active family yet. Render
                     a centered card with a primary CTA that opens the same
                     create-person drawer as the toolbar button. Clicking
                     anywhere on the card (not just the button) triggers it
                     so users who treat the whole canvas as actionable get
                     the expected result. -->
                <div
                    v-else-if="tree.data.value && tree.data.value.nodes.length === 0"
                    class="empty-state"
                    role="button"
                    tabindex="0"
                    data-testid="tree-empty"
                    @click="onCreateClick"
                    @keydown.enter="onCreateClick"
                    @keydown.space.prevent="onCreateClick"
                >
                    <v-card class="empty-card" elevation="2">
                        <v-card-title class="text-h6">{{ t('tree.empty.title') }}</v-card-title>
                        <v-card-text class="text-body-2">
                            {{ t('tree.empty.subtitle') }}
                        </v-card-text>
                        <v-card-actions class="justify-center pb-4">
                            <v-btn
                                color="primary"
                                prepend-icon="user-plus"
                                data-testid="tree-empty-cta"
                                @click.stop="onCreateClick"
                            >
                                {{ t('tree.empty.cta') }}
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </div>
                <FamilyTree
                    v-else-if="tree.data.value"
                    ref="treeRef"
                    :tree="tree.data.value"
                    :selected-id="selectedId"
                    :center-on-id="centerOnId"
                    :current-user-id="auth.user?.id ?? null"
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
.empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: calc(100vh - 200px);
    cursor: pointer;
    outline: none;
}
.empty-state:focus-visible .empty-card {
    outline: 2px solid rgb(var(--v-theme-primary));
    outline-offset: 4px;
}
.empty-card {
    max-width: 420px;
    text-align: center;
}
</style>
