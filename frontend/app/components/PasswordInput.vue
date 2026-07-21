<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { Eye, EyeOff } from '@lucide/vue'
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput } from '@/components/ui/input-group'

withDefaults(
  defineProps<{
    modelValue: string
    id?: string
    autocomplete?: string
    disabled?: boolean
    class?: HTMLAttributes['class']
  }>(),
  { autocomplete: 'current-password', disabled: false },
)

defineEmits<{ (e: 'update:modelValue', value: string): void }>()

const visible = ref(false)
</script>

<template>
  <InputGroup :class="$props.class">
    <InputGroupInput
      :id="id"
      :model-value="modelValue"
      :type="visible ? 'text' : 'password'"
      :autocomplete="autocomplete"
      :disabled="disabled"
      @update:model-value="(v) => $emit('update:modelValue', String(v))"
    />
    <InputGroupAddon align="inline-end">
      <InputGroupButton type="button" size="icon-xs" @click="visible = !visible">
        <EyeOff v-if="visible" class="size-4" />
        <Eye v-else class="size-4" />
      </InputGroupButton>
    </InputGroupAddon>
  </InputGroup>
</template>
