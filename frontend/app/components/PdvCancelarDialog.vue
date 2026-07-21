<script setup lang="ts">
import { X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Textarea } from '@/components/ui/textarea'

const emit = defineEmits<{ cancelar: [motivo: string] }>()
const visible = defineModel<boolean>('visible', { required: true })

const motivo = ref('')

watch(visible, (v) => {
    if (v) motivo.value = ''
})

function cancelar() {
    emit('cancelar', motivo.value)
}
</script>

<template>
    <Dialog v-model:open="visible">
        <DialogContent class="sm:max-w-md">
            <DialogHeader>
                <DialogTitle>Cancelar Venda</DialogTitle>
            </DialogHeader>
            <Field>
                <FieldLabel for="cancelar-motivo">Motivo</FieldLabel>
                <Textarea id="cancelar-motivo" v-model="motivo" rows="3" aria-required="true" />
            </Field>
            <DialogFooter>
                <Button variant="ghost" @click="visible = false">Voltar</Button>
                <Button variant="destructive" :disabled="!motivo" @click="cancelar">
                    <X class="size-4" />
                    Cancelar Venda
                </Button>
            </DialogFooter>
        </DialogContent>
    </Dialog>
</template>
