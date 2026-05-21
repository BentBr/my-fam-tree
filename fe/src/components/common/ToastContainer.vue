<script setup lang="ts">
import { useUiStore } from '@/stores/ui'

const ui = useUiStore()

const colorByKind = {
    info: 'info',
    success: 'success',
    error: 'error',
} as const

function onSnackbarUpdate(open: boolean, id: string): void {
    if (!open) ui.dismissToast(id)
}
</script>

<template>
    <!-- One v-snackbar per toast; stacked at bottom-right via a marginBottom
         offset. Each closes itself after its timeout or when the user clicks
         Dismiss. Pointer-events on the wrapper let the user click through the
         empty area between snackbars. -->
    <div class="toast-stack">
        <v-snackbar
            v-for="(t, idx) in ui.toasts"
            :key="t.id"
            :model-value="true"
            :color="colorByKind[t.kind]"
            :timeout="t.kind === 'error' ? 8000 : 4000"
            location="bottom right"
            :style="{ marginBottom: `${idx * 72}px` }"
            data-testid="toast"
            :data-testid-kind="t.kind"
            @update:model-value="(open: boolean) => onSnackbarUpdate(open, t.id)"
        >
            <div class="d-flex flex-column">
                <span class="font-weight-medium">{{ t.message }}</span>
                <span v-if="t.code" class="text-caption opacity-75">{{ t.code }}</span>
                <span v-if="t.requestId" class="text-caption opacity-50">id: {{ t.requestId }}</span>
            </div>
            <template #actions>
                <v-btn variant="text" data-testid="toast-dismiss" @click="ui.dismissToast(t.id)">
                    {{ $t('toast.dismiss') }}
                </v-btn>
            </template>
        </v-snackbar>
    </div>
</template>

<style scoped>
.toast-stack {
    pointer-events: none;
}
.toast-stack :deep(.v-snackbar) {
    pointer-events: auto;
}
</style>
