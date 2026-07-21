<script setup lang="ts">
import { LogOut, Menu, Moon, Sun, User } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

const props = withDefaults(
  defineProps<{
    /** Overrides for non-tenant contexts (e.g. the backoffice). */
    username?: string
    subtitle?: string
    home?: string
    logout?: () => void | Promise<void>
  }>(),
  { home: '/' },
)

const { toggleMenu, toggleDarkMode, isDarkTheme } = useLayout()
const auth = useAuth()
// Sino de notificações só faz sentido no contexto de tenant (alertas de BI).
const { isBackoffice } = useSubdomain()
const { nome: marcaNome } = useMarca()

const displayName = computed(() => props.username ?? auth.username.value)
const displaySubtitle = computed(
  () => props.subtitle ?? (auth.tenantSlug.value || auth.roles.value.join(', ')),
)

function onLogout() {
  return (props.logout ?? auth.logout)()
}
</script>

<template>
  <header class="sticky top-0 z-20 flex h-14 items-center gap-2 border-b bg-background px-3">
    <Button variant="ghost" size="icon" @click="toggleMenu">
      <Menu class="size-5" />
    </Button>

    <NuxtLink :to="home" class="flex items-center gap-2 lg:hidden">
      <AppLogoIcon class="text-primary" style="font-size: 1.5rem" />
      <span class="brand-wordmark" style="--brand-wordmark-base: 1.125rem">{{ marcaNome }}</span>
    </NuxtLink>

    <div class="ml-auto flex items-center gap-1">
      <NotificacoesBell v-if="!isBackoffice" />

      <Button variant="ghost" size="icon" aria-label="Alternar tema" @click="toggleDarkMode">
        <Moon v-if="!isDarkTheme" class="size-5" />
        <Sun v-else class="size-5" />
      </Button>

      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button variant="ghost" class="hidden items-center gap-2 lg:flex">
            <User class="size-4" />
            <span class="flex flex-col items-start leading-tight">
              <span class="font-medium text-sm">{{ displayName || 'Usuário' }}</span>
              <span class="text-xs text-muted-foreground">{{ displaySubtitle }}</span>
            </span>
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem @click="onLogout">
            <LogOut class="size-4" />
            Sair
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  </header>
</template>
