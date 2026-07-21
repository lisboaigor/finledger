<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { InputGroup, InputGroupAddon, InputGroupInput, InputGroupText } from '@/components/ui/input-group'

const props = withDefaults(
  defineProps<{
    modelValue: number | null
    min?: number
    max?: number
    disabled?: boolean
    id?: string
    class?: HTMLAttributes['class']
  }>(),
  { min: undefined, max: undefined, disabled: false },
)

const emit = defineEmits<{ (e: 'update:modelValue', value: number | null): void }>()

const fmt = new Intl.NumberFormat('pt-BR', { minimumFractionDigits: 2, maximumFractionDigits: 2 })

const focused = ref(false)
const display = ref(props.modelValue != null ? fmt.format(props.modelValue) : '')

watch(
  () => props.modelValue,
  (v) => {
    if (!focused.value) display.value = v != null ? fmt.format(v) : ''
  },
)

function parse(raw: string): number | null {
  const cleaned = raw.replace(/[^\d,.-]/g, '').replace(/\./g, '').replace(',', '.')
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
  display.value = n != null ? fmt.format(n) : ''
  emit('update:modelValue', n)
}
</script>

<template>
  <InputGroup :class="props.class">
    <InputGroupAddon>
      <InputGroupText>R$</InputGroupText>
    </InputGroupAddon>
    <InputGroupInput
      :id="id"
      v-model="display"
      inputmode="decimal"
      :disabled="disabled"
      @focus="onFocus"
      @blur="onBlur"
    />
  </InputGroup>
</template>
