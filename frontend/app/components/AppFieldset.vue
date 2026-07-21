<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { FieldLegend, FieldSet } from '@/components/ui/field'

defineProps<{
  legend: string
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <FieldSet :class="['rounded-lg border p-4', $props.class]">
    <FieldLegend class="px-1">
      <slot name="legend">{{ legend }}</slot>
    </FieldLegend>
    <slot />
  </FieldSet>
</template>
