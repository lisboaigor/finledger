<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { Minus, Plus } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

const props = withDefaults(
  defineProps<{
    modelValue: number
    min?: number
    max?: number
    step?: number
    disabled?: boolean
    class?: HTMLAttributes['class']
  }>(),
  { min: 0, max: undefined, step: 1, disabled: false },
)

const emit = defineEmits<{ (e: 'update:modelValue', value: number): void }>()

function clamp(n: number) {
  let v = n
  if (props.min != null) v = Math.max(props.min, v)
  if (props.max != null) v = Math.min(props.max, v)
  return v
}

function set(n: number) {
  emit('update:modelValue', clamp(n))
}

function onInput(e: Event) {
  const raw = (e.target as HTMLInputElement).value
  const n = Number(raw.replace(/[^\d-]/g, ''))
  if (!Number.isNaN(n)) set(n)
}
</script>

<template>
  <div :class="['flex items-center gap-1', props.class]">
    <Button
      type="button"
      variant="outline"
      size="icon-sm"
      :disabled="disabled || (min != null && modelValue <= min)"
      @click="set(modelValue - step)"
    >
      <Minus class="size-3.5" />
    </Button>
    <Input
      :model-value="modelValue"
      :disabled="disabled"
      inputmode="numeric"
      class="w-14 text-center"
      @input="onInput"
    />
    <Button
      type="button"
      variant="outline"
      size="icon-sm"
      :disabled="disabled || (max != null && modelValue >= max)"
      @click="set(modelValue + step)"
    >
      <Plus class="size-3.5" />
    </Button>
  </div>
</template>
