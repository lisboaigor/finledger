<script setup lang="ts">
import { Building2, ChartLine, Users } from '@lucide/vue'
import { Toaster } from '@/components/ui/sonner'
import type { MenuGroup } from '~/components/AppMenu.vue'

// Same chrome as the tenant app (see layouts/default.vue) with a
// backoffice-specific menu and auth context.
const { isDarkTheme } = useLayout()
const { username, hasPermission, logout } = useBackofficeAuth()

const groups = computed<MenuGroup[]>(() => [
  {
    label: 'Plataforma',
    items: [
      { label: 'Visão Geral', icon: ChartLine, to: '/dashboard' },
      { label: 'Tenants', icon: Building2, to: '/tenants' },
      ...(hasPermission('admins:manage') ? [{ label: 'Admins', icon: Users, to: '/admins' }] : []),
    ],
  },
])
</script>

<template>
  <div class="flex min-h-screen">
    <AppSidebar :groups="groups" />
    <div class="flex min-h-screen flex-1 flex-col">
      <AppTopbar :username="username" subtitle="Backoffice" home="/dashboard" :logout="logout" />
      <main class="flex-1 p-4 lg:p-6">
        <slot />
      </main>
      <AppFooter />
    </div>
  </div>
  <Toaster :theme="isDarkTheme ? 'dark' : 'light'" position="top-right" rich-colors />
  <ConfirmDialog />
</template>
