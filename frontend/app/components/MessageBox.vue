<script setup lang="ts">
import { AlertTriangle, CheckCircle2, Info, XCircle } from '@lucide/vue'
import { Alert, AlertDescription } from '@/components/ui/alert'

/** Substitui `<Message :severity>` do PrimeVue — texto de aviso inline (não é toast). */
withDefaults(
  defineProps<{
    severity?: 'success' | 'info' | 'warn' | 'error'
  }>(),
  { severity: 'info' },
)

const classesBySeverity = {
  success: 'border-emerald-200 bg-emerald-50 text-emerald-900 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-300 [&_svg]:text-emerald-600 dark:[&_svg]:text-emerald-400',
  info: 'border-sky-200 bg-sky-50 text-sky-900 dark:border-sky-500/30 dark:bg-sky-500/10 dark:text-sky-300 [&_svg]:text-sky-600 dark:[&_svg]:text-sky-400',
  warn: 'border-amber-200 bg-amber-50 text-amber-900 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-300 [&_svg]:text-amber-600 dark:[&_svg]:text-amber-400',
  error: 'border-red-200 bg-red-50 text-red-900 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-300 [&_svg]:text-red-600 dark:[&_svg]:text-red-400',
} as const

const iconsBySeverity = {
  success: CheckCircle2,
  info: Info,
  warn: AlertTriangle,
  error: XCircle,
}
</script>

<template>
  <Alert :class="classesBySeverity[severity]">
    <component :is="iconsBySeverity[severity]" class="size-4" />
    <AlertDescription class="text-current">
      <slot />
    </AlertDescription>
  </Alert>
</template>
