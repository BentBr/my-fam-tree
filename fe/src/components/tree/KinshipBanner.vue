<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { KinshipRelation } from '@/api/types'

import { kinshipLabel } from './kinshipLabel'

const props = defineProps<{
    nameA: string
    nameB: string
    /** B's gender — every label names B. */
    genderB: string
    /** How many persons are selected: 0, 1 or 2. */
    selectedCount: number
    /** Relations of the selected pair; null while there is no result. */
    relations: KinshipRelation[] | null
    loading: boolean
    /** The request failed and there is no cached result to show instead. */
    failed: boolean
}>()

const emit = defineEmits<{
    /** The relation whose path the canvas should light, or null. */
    (e: 'active-relation', relation: KinshipRelation | null): void
    (e: 'retry'): void
}>()

const { t } = useI18n()

const activeIndex = ref(0)
watch(
    () => props.relations,
    () => {
        activeIndex.value = 0
    },
)

const labels = computed<string[]>(() =>
    (props.relations ?? []).map((relation) => kinshipLabel(relation, props.genderB, (key, named) => t(key, named))),
)
const activeLabel = computed<string | null>(() => labels.value[activeIndex.value] ?? null)
const active = computed<KinshipRelation | null>(() => props.relations?.[activeIndex.value] ?? null)
watch(
    active,
    (relation) => {
        emit('active-relation', relation)
    },
    { immediate: true },
)
</script>

<template>
    <v-card class="kinship-banner" elevation="4" data-testid="kinship-banner">
        <v-card-text class="py-2 px-4">
            <span v-if="selectedCount === 0" data-testid="kinship-hint">
                {{ t('tree.kinship.banner.selectFirst') }}
            </span>
            <span v-else-if="selectedCount === 1" data-testid="kinship-hint">
                {{ t('tree.kinship.banner.selectSecond') }}
            </span>
            <span v-else-if="loading" data-testid="kinship-loading">
                <v-progress-circular indeterminate size="16" class="mr-2" />
                {{ t('tree.kinship.banner.loading') }}
            </span>
            <span v-else-if="failed" data-testid="kinship-error">
                {{ t('tree.kinship.banner.error') }}
                <v-btn size="small" variant="text" data-testid="kinship-retry" @click="emit('retry')">
                    {{ t('tree.kinship.banner.retry') }}
                </v-btn>
            </span>
            <span v-else-if="relations !== null && relations.length === 0" data-testid="kinship-none">
                {{ t('tree.kinship.banner.none') }}
            </span>
            <template v-else-if="activeLabel !== null">
                <i18n-t keypath="tree.kinship.banner.sentence" tag="span" scope="global" data-testid="kinship-sentence">
                    <template #a>
                        <strong>{{ nameA }}</strong>
                    </template>
                    <template #b>
                        <strong>{{ nameB }}</strong>
                    </template>
                    <template #relation>
                        <strong>{{ activeLabel }}</strong>
                    </template>
                </i18n-t>
                <div v-if="labels.length > 1" class="mt-1">
                    <v-chip
                        v-for="(label, i) in labels"
                        :key="i"
                        size="small"
                        class="mr-1"
                        :color="i === activeIndex ? 'primary' : undefined"
                        :data-testid="`kinship-chip-${String(i)}`"
                        @click="activeIndex = i"
                    >
                        {{ label }}
                    </v-chip>
                </div>
            </template>
        </v-card-text>
    </v-card>
</template>

<style scoped>
.kinship-banner {
    position: absolute;
    top: 12px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 4;
    max-width: min(90%, 560px);
}
</style>
