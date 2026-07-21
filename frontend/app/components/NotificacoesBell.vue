<script setup lang="ts">
/** Sino de notificações do topbar: badge com a contagem de recomendações do
 * motor de BI e um painel compacto para agir (abrir/resolver/silenciar) sem
 * poluir o dashboard. */
import { ArrowRight, Bell, Check, X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'

const notif = useNotificacoes()

onMounted(() => notif.carregar())

function abrir(link: string) {
  navigateTo(link)
}
</script>

<template>
  <Popover>
    <PopoverTrigger as-child>
      <Button variant="ghost" size="icon" class="relative" aria-label="Notificações">
        <Bell class="size-5" />
        <span
          v-if="notif.alertas.length"
          class="absolute right-1 top-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-red-500 px-1 text-[0.625rem] font-bold text-white"
        >
          {{ notif.alertas.length > 9 ? '9+' : notif.alertas.length }}
        </span>
      </Button>
    </PopoverTrigger>
    <PopoverContent align="end" class="w-[26rem] max-w-[92vw]">
      <div class="mb-2 flex items-center justify-between">
        <span class="font-semibold">Notificações</span>
        <StatusBadge v-if="notif.alertas.length" :value="notif.alertas.length" severity="warn" />
      </div>

      <p v-if="!notif.alertas.length" class="py-4 text-center text-sm text-muted-foreground">
        Nada pendente. 🎉
      </p>

      <ul v-else class="flex max-h-96 flex-col overflow-y-auto">
        <li
          v-for="alerta in notif.alertas"
          :key="alerta.alerta_id"
          class="flex items-start gap-2.5 py-2.5 first:pt-0 [&:not(:first-child)]:border-t"
        >
          <component :is="notif.iconeAlerta(alerta.codigo)" class="mt-0.5 size-4 shrink-0 text-primary" />
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium leading-snug">{{ alerta.titulo }}</p>
            <p class="line-clamp-2 text-xs leading-snug text-muted-foreground">{{ alerta.mensagem }}</p>
          </div>
          <div class="flex shrink-0 items-center">
            <Button variant="ghost" size="icon-sm" title="Abrir" @click="abrir(alerta.link)">
              <ArrowRight class="size-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              title="Resolvido"
              class="text-emerald-600"
              :disabled="notif.enviandoFeedback === alerta.alerta_id"
              @click="notif.feedback(alerta, 'resolvido')"
            >
              <Check class="size-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              title="Silenciar por 30 dias"
              :disabled="notif.enviandoFeedback === alerta.alerta_id"
              @click="notif.feedback(alerta, 'ignorado')"
            >
              <X class="size-4" />
            </Button>
          </div>
        </li>
      </ul>
    </PopoverContent>
  </Popover>
</template>
