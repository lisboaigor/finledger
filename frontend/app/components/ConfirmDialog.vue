<script setup lang="ts">
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'

const state = useConfirmState()

async function onAccept() {
  await state.options?.accept()
  state.visible = false
}

function onReject() {
  state.options?.reject?.()
  state.visible = false
}
</script>

<template>
  <AlertDialog v-model:open="state.visible">
    <AlertDialogContent v-if="state.options">
      <AlertDialogHeader>
        <AlertDialogTitle>{{ state.options.header ?? 'Confirmar' }}</AlertDialogTitle>
        <AlertDialogDescription>{{ state.options.message }}</AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel @click="onReject">
          {{ state.options.rejectLabel ?? 'Cancelar' }}
        </AlertDialogCancel>
        <AlertDialogAction
          :variant="state.options.variant === 'danger' ? 'destructive' : 'default'"
          @click="onAccept"
        >
          {{ state.options.acceptLabel ?? 'Confirmar' }}
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
