<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import LangSwitcher from '@/components/layout/LangSwitcher.vue'

const { t } = useI18n()
</script>

<template>
    <v-main class="login-main">
        <v-container class="d-flex align-center justify-center" fluid>
            <div class="login-frame">
                <header class="d-flex align-center justify-space-between mb-6">
                    <h1 class="text-h5">{{ t('app.title') }}</h1>
                    <LangSwitcher />
                </header>
                <div class="fade-router-view">
                    <router-view v-slot="{ Component, route }">
                        <transition name="fade" mode="out-in" appear>
                            <component :is="Component" :key="route.fullPath" />
                        </transition>
                    </router-view>
                </div>
            </div>
        </v-container>
    </v-main>
</template>

<style scoped>
.login-main {
    min-height: 100vh;
    background:
        radial-gradient(1200px 600px at 10% -10%, rgb(var(--v-theme-primary) / 0.12), transparent 60%),
        radial-gradient(900px 500px at 90% 110%, rgb(var(--v-theme-secondary) / 0.1), transparent 60%),
        rgb(var(--v-theme-background));
}
.login-frame {
    width: 100%;
    max-width: 480px;
}
</style>
