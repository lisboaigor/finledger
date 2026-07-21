<script setup lang="ts">
import { Badge } from '@/components/ui/badge'

/** Substitui `<Tag :severity>` do PrimeVue — mesmo vocabulário de severidade. */
export type BadgeSeverity = 'success' | 'info' | 'warn' | 'danger' | 'error' | 'secondary' | 'contrast'

const props = defineProps<{
  value: string | number
  severity?: BadgeSeverity
}>()

const classesBySeverity: Record<BadgeSeverity, string> = {
  success: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-500/15 dark:text-emerald-300',
  info: 'bg-sky-100 text-sky-800 dark:bg-sky-500/15 dark:text-sky-300',
  warn: 'bg-amber-100 text-amber-800 dark:bg-amber-500/15 dark:text-amber-300',
  danger: 'bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-300',
  error: 'bg-red-100 text-red-800 dark:bg-red-500/15 dark:text-red-300',
  secondary: '',
  contrast: 'bg-foreground text-background',
}

const classes = computed(() => classesBySeverity[props.severity ?? 'secondary'])
</script>

<template>
  <Badge :variant="severity === 'secondary' || !severity ? 'secondary' : 'outline'" :class="classes">
    {{ value }}
  </Badge>
</template>
