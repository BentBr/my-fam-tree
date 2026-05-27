<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useDisplay } from 'vuetify'

import { useSetFavourite } from '@/api/hooks/persons'
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
const { smAndDown } = useDisplay()

const pageTitle = computed(() => {
    const name = family.activeFamily?.name
    if (name === undefined || name === '') return t('tree.title')
    return t('tree.titleOf', { name })
})

// URL `?center=<personId>` is the SOURCE OF TRUTH for both the tree's
// center and the drawer's open state. We don't strip the param after
// reading it — the URL stays in sync with the current selection, which
// makes hard-reloads and back/forward survive, and (critically) frees
// us from the timing acrobatics required to flip selectedId at exactly
// the right tick. TreeNode clicks update the URL via `router.replace`
// instead of mutating selectedId directly; the watcher below feeds the
// URL change back into selectedId.
const centerFromUrl = computed<string | null>(() => {
    const val = route.query['center']
    return typeof val === 'string' && val !== '' ? val : null
})

const selectedId = ref<string | null>(null)
const creating = ref(false)

/**
 * Center-on-member resolution order:
 *   1. ?center=<personId> in the URL (deep link or persisted selection).
 *   2. useActiveFamilyStore.focusedPersonId (legacy persisted choice).
 *   3. The TreeNode whose `linked_user_id` equals the signed-in user.id.
 *   4. null — FamilyTree falls back to the layout origin + 40px gutter.
 */
const centerOnId = computed<string | null>(() => {
    if (centerFromUrl.value !== null) return centerFromUrl.value
    if (family.focusedPersonId !== null) return family.focusedPersonId
    const userId = auth.user?.id ?? null
    if (userId === null) return null
    const me = tree.data.value?.nodes.find(
        (n) => n.linked_user_id !== null && n.linked_user_id !== undefined && n.linked_user_id === userId,
    )
    return me?.id ?? null
})

function selectInUrl(id: string | null): void {
    const rest = { ...route.query }
    if (id === null) {
        delete rest['center']
    } else {
        rest['center'] = id
    }
    void router.replace({ query: rest })
}

function onSelect(id: string): void {
    // Set selectedId synchronously so the drawer responds in the same
    // tick — same edge Vuetify has always honored for the on-click
    // path. The URL update is async (router.replace) and the watcher
    // will see no diff afterwards (target === selectedId.value).
    selectedId.value = id
    family.setFocusedPerson(id)
    selectInUrl(id)
}

function closeDrawer(): void {
    selectedId.value = null
    creating.value = false
    selectInUrl(null)
}

function onDrawerUpdate(open: boolean): void {
    if (!open) closeDrawer()
}

function onCreateClick(): void {
    creating.value = true
    selectedId.value = null
    selectInUrl(null)
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

const setFavourite = useSetFavourite()
function onToggleFavourite(id: string, next: boolean): void {
    // The hook itself owns optimistic update + rollback; we just kick it.
    setFavourite.mutate({ id, isFavourite: next })
}

// Mirror `?center=` into `selectedId` reactively. Two-phase so Vuetify's
// `v-navigation-drawer` (temporary) sees a clean `false → true` edge:
// the drawer mounts with `selectedId=null` (model-value=false), then
// the watcher flips it post-mount. The `isMounted` gate prevents the
// flip from happening during setup, which is the case the previous
// implementations kept tripping over — Vuetify silently no-ops a
// model-value already true at first paint.
const isMounted = ref(false)
onMounted(() => {
    isMounted.value = true
})

watch([isMounted, centerFromUrl] as const, ([mounted, target]) => {
    if (!mounted) return
    if (target === selectedId.value) return
    selectedId.value = target
    if (target !== null) family.setFocusedPerson(target)
})
</script>

<template>
    <div class="tree-page">
        <!-- The heading + actions share one toolbar row. On phones the family
             name can be long, so the title truncates (it never pushes the
             actions off-screen) and shrinks a step, while the action buttons
             collapse to icon-only to reclaim horizontal space. Desktop keeps
             the roomier heading and labelled buttons. -->
        <v-toolbar density="comfortable" elevation="0" color="transparent">
            <v-toolbar-title
                class="tree-title"
                :class="smAndDown ? 'text-h6' : 'text-h5'"
                data-testid="tree-page-title"
                :title="pageTitle"
            >
                {{ pageTitle }}
            </v-toolbar-title>
            <v-spacer />
            <v-btn
                v-if="tree.data.value !== undefined && tree.data.value.nodes.length > 0"
                :icon="smAndDown ? 'maximize' : undefined"
                :prepend-icon="smAndDown ? undefined : 'maximize'"
                variant="text"
                :title="t('tree.fitToView')"
                data-testid="tree-fit-to-view"
                @click="onFit"
            >
                <template v-if="!smAndDown">{{ t('tree.fitToView') }}</template>
            </v-btn>
            <v-btn
                :icon="smAndDown ? 'user-plus' : undefined"
                :prepend-icon="smAndDown ? undefined : 'user-plus'"
                color="primary"
                :title="t('tree.addPerson')"
                data-testid="tree-add-person"
                @click="onCreateClick"
            >
                <template v-if="!smAndDown">{{ t('tree.addPerson') }}</template>
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
                    @toggle-favourite="onToggleFavourite"
                />
            </div>

            <v-navigation-drawer
                :model-value="selectedId !== null || creating"
                location="right"
                :width="380"
                temporary
                disable-route-watcher
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
/* Let the title shrink and ellipsis-truncate inside the flex toolbar instead
 * of pushing the action buttons past the viewport edge on narrow screens.
 * `min-width: 0` is required for a flex child to be allowed to shrink below
 * its content width; :deep targets the inner element Vuetify renders the
 * title text into. */
.tree-title {
    min-width: 0;
}
.tree-title :deep(.v-toolbar-title__placeholder) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
