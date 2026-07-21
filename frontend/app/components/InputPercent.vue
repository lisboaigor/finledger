<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { InputGroup, InputGroupAddon, InputGroupInput, InputGroupText } from '@/components/ui/input-group'

const props = withDefaults(
  defineProps<{
    modelValue: number | null
    min?: number
    max?: number
    maxFractionDigits?: number
    disabled?: boolean
    id?: string
    class?: HTMLAttributes['class']
  }>(),
  { min: undefined, max: undefined, maxFractionDigits: 2, disabled: false },
)

const emit = defineEmits<{ (e: 'update:modelValue', value: number | null): void }>()

const fmt = computed(() => new Intl.NumberFormat('pt-BR', { maximumFractionDigits: props.maxFractionDigits }))

const focused = ref(false)
const display = ref(props.modelValue != null ? fmt.value.format(props.modelValue) : '')

watch(
  () => props.modelValue,
  (v) => {
    if (!focused.value) display.value = v != null ? fmt.value.format(v) : ''
  },
)

function parse(raw: string): number | null {
  const cleaned = raw.replace(/[^\d,-]/g, '').replace(',', '.')
  if (!cleaned) return null
  const n = Number(cleaned)
  return Number.isNaN(n) ? null : n
}

function onFocus() {
  focused.value = true
}

function onBlur() {
  focused.value = false
  let n = parse(display.value)
  if (n != null) {
    if (props.min != null) n = Math.max(props.min, n)
    if (props.max != null) n = Math.min(props.max, n)
  }
  display.value = n != null ? fmt.value.format(n) : ''
  emit('update:modelValue', n)
}
</script>

<template>
  <InputGroup :class="props.class">
    <InputGroupInput
      :id="id"
      v-model="display"
      inputmode="decimal"
      :disabled="disabled"
      @focus="onFocus"
      @blur="onBlur"
    />
    <InputGroupAddon align="inline-end">
      <InputGroupText>%</InputGroupText>
    </InputGroupAddon>
  </InputGroup>
</template>
