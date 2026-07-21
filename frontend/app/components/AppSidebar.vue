<script setup lang="ts">
import type { MenuGroup } from '~/components/AppMenu.vue'

defineProps<{ groups?: MenuGroup[] }>()

const { layoutState, hideMobileMenu } = useLayout()
const { nome: marcaNome } = useMarca()
</script>

<template>
  <aside
    class="fixed inset-y-0 left-0 z-40 w-64 shrink-0 border-r bg-sidebar text-sidebar-foreground transition-transform duration-200 lg:sticky lg:top-0 lg:h-screen lg:translate-x-0"
    :class="[
      layoutState.mobileMenuActive ? 'translate-x-0' : '-translate-x-full',
      layoutState.staticMenuInactive && 'lg:hidden',
    ]"
  >
    <div class="flex h-14 items-center gap-2 border-b px-4">
      <AppLogoIcon class="text-primary" style="font-size: 1.75rem" />
      <span class="brand-wordmark truncate" style="--brand-wordmark-base: 1.25rem">{{ marcaNome }}</span>
    </div>
    <div class="h-[calc(100vh-3.5rem)] overflow-y-auto">
      <AppMenu :groups="groups" />
    </div>
  </aside>
  <div
    v-if="layoutState.mobileMenuActive"
    class="fixed inset-0 z-30 bg-black/40 lg:hidden"
    @click="hideMobileMenu"
  />
</template>
