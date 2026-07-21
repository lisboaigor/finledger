<script setup lang="ts">
import { Check, ChevronsUpDown, X } from '@lucide/vue'
import {
  Combobox,
  ComboboxAnchor,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxItemIndicator,
  ComboboxList,
  ComboboxTrigger,
  ComboboxViewport,
} from '@/components/ui/combobox'

export interface SearchSelectOption {
  label: string
  value: string | number | null
}

const props = withDefaults(
  defineProps<{
    modelValue: string | number | null | undefined
    options: SearchSelectOption[]
    placeholder?: string
    clearable?: boolean
    disabled?: boolean
    class?: string
  }>(),
  { placeholder: 'Selecione', clearable: false, disabled: false },
)

const emit = defineEmits<{ (e: 'update:modelValue', value: string | number | null): void }>()

const selectedLabel = computed(
  () => props.options.find((o) => o.value === props.modelValue)?.label ?? '',
)

function onSelect(value: string | number | null) {
  emit('update:modelValue', value)
}

function clear(e: Event) {
  e.stopPropagation()
  emit('update:modelValue', null)
}
</script>

<template>
  <Combobox :model-value="modelValue" :disabled="disabled" @update:model-value="onSelect">
    <ComboboxAnchor :class="['w-full', props.class]">
      <ComboboxTrigger as-child>
        <button
          type="button"
          class="border-input bg-background dark:bg-input/30 dark:hover:bg-input/50 focus-visible:border-ring focus-visible:ring-ring/50 flex h-8 w-full items-center justify-between gap-1.5 rounded-lg border px-2.5 py-2 text-sm outline-none focus-visible:ring-3 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <span :class="['truncate', !selectedLabel && 'text-muted-foreground']">
            {{ selectedLabel || placeholder }}
          </span>
          <span class="flex items-center gap-1 shrink-0">
            <X v-if="clearable && modelValue != null" class="size-3.5 text-muted-foreground hover:text-foreground" @click="clear" />
            <ChevronsUpDown class="text-muted-foreground size-4" />
          </span>
        </button>
      </ComboboxTrigger>
    </ComboboxAnchor>
    <ComboboxList>
      <ComboboxInput :placeholder="`Buscar ${placeholder.toLowerCase()}...`" />
      <ComboboxEmpty>Nenhum resultado.</ComboboxEmpty>
      <ComboboxViewport>
        <ComboboxItem v-for="opt in options" :key="String(opt.value)" :value="opt.value">
          {{ opt.label }}
          <ComboboxItemIndicator class="absolute right-2 flex items-center">
            <Check class="size-4" />
          </ComboboxItemIndicator>
        </ComboboxItem>
      </ComboboxViewport>
    </ComboboxList>
  </Combobox>
</template>
