<script setup lang="ts">
import { Toaster } from '@/components/ui/sonner'

const { isDarkTheme } = useLayout()
</script>

<template>
  <div class="flex min-h-screen">
    <AppSidebar />
    <div class="flex min-h-screen flex-1 flex-col">
      <AppTopbar />
      <main class="flex-1 p-4 lg:p-6">
        <slot />
      </main>
      <AppFooter />
    </div>
  </div>
  <Toaster :theme="isDarkTheme ? 'dark' : 'light'" position="top-right" rich-colors />
  <ConfirmDialog />
</template>
