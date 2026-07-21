<script setup lang="ts">
import { X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Textarea } from '@/components/ui/textarea'

const emit = defineEmits<{ recusar: [motivo: string] }>()
const visible = defineModel<boolean>('visible', { required: true })

const motivo = ref('')

watch(visible, (v) => {
    if (v) motivo.value = ''
})

function recusar() {
    emit('recusar', motivo.value)
}
</script>

<template>
    <Dialog v-model:open="visible">
        <DialogContent class="sm:max-w-md">
            <DialogHeader>
                <DialogTitle>Recusar Orçamento</DialogTitle>
            </DialogHeader>
            <Field>
                <FieldLabel>Motivo da recusa</FieldLabel>
                <Textarea v-model="motivo" rows="3" />
            </Field>
            <DialogFooter>
                <Button variant="ghost" @click="visible = false">Voltar</Button>
                <Button variant="destructive" :disabled="!motivo" @click="recusar">
                    <X class="size-4" />
                    Recusar
                </Button>
            </DialogFooter>
        </DialogContent>
    </Dialog>
</template>
