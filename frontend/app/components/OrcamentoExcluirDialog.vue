<script setup lang="ts">
import { Trash2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Textarea } from '@/components/ui/textarea'

const emit = defineEmits<{ excluir: [motivo: string] }>()
const visible = defineModel<boolean>('visible', { required: true })

const motivo = ref('')

watch(visible, (v) => {
    if (v) motivo.value = ''
})

function excluir() {
    emit('excluir', motivo.value)
}
</script>

<template>
    <Dialog v-model:open="visible">
        <DialogContent class="sm:max-w-md">
            <DialogHeader>
                <DialogTitle>Excluir Orçamento</DialogTitle>
            </DialogHeader>
            <Field>
                <FieldLabel>Motivo da exclusão</FieldLabel>
                <Textarea v-model="motivo" rows="3" />
            </Field>
            <DialogFooter>
                <Button variant="ghost" @click="visible = false">Voltar</Button>
                <Button variant="destructive" :disabled="!motivo" @click="excluir">
                    <Trash2 class="size-4" />
                    Excluir
                </Button>
            </DialogFooter>
        </DialogContent>
    </Dialog>
</template>
