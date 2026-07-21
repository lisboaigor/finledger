<template>
  <NuxtRouteAnnouncer />
  <NuxtLayout>
    <NuxtPage />
  </NuxtLayout>
</template>

<script setup lang="ts">
// The dark-mode class lives on <html> and is driven reactively from the
// persisted theme, so SSR already renders the right theme (no flash) and
// reloads keep the user's choice.
const { isDarkTheme } = useLayout()

useHead({
  htmlAttrs: {
    class: computed(() => (isDarkTheme.value ? 'app-dark' : '')),
  },
})
</script>
