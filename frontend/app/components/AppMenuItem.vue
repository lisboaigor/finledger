<script setup lang="ts">
import type { MenuEntry } from '~/components/AppMenu.vue'

const props = defineProps<{ item: MenuEntry }>()

const route = useRoute()
const { hideMobileMenu } = useLayout()

const isActive = computed(() => {
  if (!props.item.to) return false
  if (props.item.to === '/') return route.path === '/'
  return route.path.startsWith(props.item.to)
})
</script>

<template>
  <NuxtLink
    :to="item.to"
    class="flex items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-sm transition-colors"
    :class="isActive
      ? 'bg-primary/10 text-primary font-medium'
      : 'text-foreground/80 hover:bg-muted hover:text-foreground'"
    @click="hideMobileMenu"
  >
    <component :is="item.icon" class="size-4 shrink-0" />
    <span class="truncate">{{ item.label }}</span>
  </NuxtLink>
</template>
