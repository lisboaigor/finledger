<script setup lang="ts">
import { CheckCircle2, Plus, Printer } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'

withDefaults(
    defineProps<{
        totalFinalizado: number
        titulo?: string
        mensagem?: string
        rotuloNovo?: string
        rotuloImprimir?: string
    }>(),
    {
        titulo: 'Venda Finalizada!',
        mensagem: 'Venda registrada com sucesso.',
        rotuloNovo: 'Nova Venda (F1)',
        rotuloImprimir: 'Imprimir Cupom',
    },
)
defineEmits<{ 'nova-venda': []; imprimir: [] }>()

const visible = defineModel<boolean>('visible', { required: true })
const { formatCentavos } = useFormat()
</script>

<template>
    <Dialog v-model:open="visible">
        <DialogContent class="sm:max-w-sm">
            <DialogHeader>
                <DialogTitle>{{ titulo }}</DialogTitle>
            </DialogHeader>
            <div class="py-6 text-center" role="status" aria-live="assertive">
                <div class="mx-auto mb-6 flex size-20 items-center justify-center rounded-full bg-emerald-100 dark:bg-emerald-500/15">
                    <CheckCircle2 class="size-12 text-emerald-600 dark:text-emerald-400" aria-hidden="true" />
                </div>
                <p class="pdv-sucesso-valor">
                    {{ formatCentavos(totalFinalizado) }}
                </p>
                <p class="mt-2 text-muted-foreground">{{ mensagem }}</p>
            </div>
            <DialogFooter>
                <div class="flex w-full flex-col gap-2">
                    <Button variant="outline" class="w-full" @click="$emit('imprimir')">
                        <Printer class="size-4" />
                        {{ rotuloImprimir }}
                    </Button>
                    <Button class="w-full" aria-keyshortcuts="F1" @click="$emit('nova-venda')">
                        <Plus class="size-4" />
                        {{ rotuloNovo }}
                    </Button>
                </div>
            </DialogFooter>
        </DialogContent>
    </Dialog>
</template>

<style scoped>
.pdv-sucesso-valor {
    font-size: clamp(2rem, 5vw, 2.75rem);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--primary);
}
</style>
