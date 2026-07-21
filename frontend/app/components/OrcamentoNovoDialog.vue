<script setup lang="ts">
import { Check } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import type { Opcao } from '~/models/shared'

defineProps<{ opcoesCliente: Opcao[] }>()
const emit = defineEmits<{
    criar: [payload: { cliente_id: string | null; cliente_avulso: string | null; validade_dias: number }]
}>()

const visible = defineModel<boolean>('visible', { required: true })

const clienteId = ref<string | null>(null)
const clienteAvulso = ref('')
const validadeDias = ref(15)

watch(visible, (v) => {
    if (v) {
        clienteId.value = null
        clienteAvulso.value = ''
        validadeDias.value = 15
    }
})

// Selecionar um cliente do CRM e digitar um nome avulso são mutuamente
// exclusivos — escolher um cadastro limpa o texto livre e vice-versa,
// espelhando a regra de domínio (IdentificacaoCliente::resolver).
watch(clienteId, (v) => {
    if (v) clienteAvulso.value = ''
})

function criar() {
    emit('criar', {
        cliente_id: clienteId.value,
        cliente_avulso: clienteAvulso.value.trim() || null,
        validade_dias: validadeDias.value,
    })
}
</script>

<template>
    <Dialog v-model:open="visible">
        <DialogContent class="sm:max-w-md">
            <DialogHeader>
                <DialogTitle>Novo Orçamento</DialogTitle>
            </DialogHeader>
            <div class="flex flex-col gap-4">
                <Field>
                    <FieldLabel>Cliente (opcional)</FieldLabel>
                    <SearchSelect
                        v-model="clienteId"
                        :options="opcoesCliente"
                        placeholder="Sem cliente"
                        clearable
                    />
                </Field>
                <Field>
                    <FieldLabel>Ou nome do cliente de balcão (sem cadastro)</FieldLabel>
                    <Input
                        v-model="clienteAvulso"
                        placeholder="Ex.: João, cliente de balcão"
                        :disabled="!!clienteId"
                        maxlength="120"
                    />
                </Field>
                <Field>
                    <FieldLabel>Validade (dias)</FieldLabel>
                    <InputQuantity v-model="validadeDias" :min="1" />
                </Field>
            </div>
            <DialogFooter>
                <Button variant="ghost" @click="visible = false">Cancelar</Button>
                <Button @click="criar">
                    <Check class="size-4" />
                    Criar
                </Button>
            </DialogFooter>
        </DialogContent>
    </Dialog>
</template>
