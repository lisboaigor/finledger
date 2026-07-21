<script setup lang="ts">
import { Clock, RefreshCw, X, XCircle } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Textarea } from '@/components/ui/textarea'

const { formatCentavos } = useFormat()
const vm = useFiscalViewModel()

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Notas Fiscais</h1>
            <p class="text-muted-foreground">Documentos fiscais emitidos.</p>
        </div>

        <AppDataTable
            :rows="vm.notas"
            :loading="vm.loading"
            row-key="nf_id"
            empty-text="Nenhuma nota fiscal."
            search-placeholder="Buscar nota..."
            :search-fields="['modelo', 'serie', 'numero', 'chave', 'status']"
            :columns="[
                { key: 'modelo', label: 'Modelo' },
                { key: 'serie_numero', label: 'Série / Número' },
                { key: 'chave', label: 'Chave' },
                { key: 'total', label: 'Total' },
                { key: 'status', label: 'Status' },
                { key: 'acoes', label: 'Ações', class: 'w-36' },
            ]"
        >
            <template #cell-serie_numero="{ row }">{{ row.serie }} / {{ row.numero }}</template>
            <template #cell-chave="{ row }">
                <span class="text-xs font-mono">{{ row.chave || '—' }}</span>
            </template>
            <template #cell-total="{ row }">{{ formatCentavos(row.total_centavos) }}</template>
            <template #cell-status="{ row }">
                <div class="flex flex-wrap gap-1">
                    <StatusBadge :value="row.status" :severity="vm.statusSeverity(row.status)" />
                    <span
                        v-if="row.cancelamento_pendente"
                        title="Devolução registrada — a nota será cancelada quando a integração com a SEFAZ estiver ativa."
                        class="inline-flex items-center gap-1"
                    >
                        <Clock class="size-3.5 text-amber-600" />
                        <StatusBadge value="Cancelamento pendente" severity="warn" />
                    </span>
                </div>
            </template>
            <template #cell-acoes="{ row }">
                <div class="flex gap-1">
                    <Button
                        v-if="['Gerada', 'Rejeitada'].includes(row.status)"
                        variant="ghost"
                        size="icon-sm"
                        title="Retransmitir"
                        @click="vm.retransmitir(row)"
                    >
                        <RefreshCw class="size-4" />
                    </Button>
                    <Button
                        v-if="row.status === 'Autorizada'"
                        variant="ghost"
                        size="icon-sm"
                        class="text-destructive"
                        title="Cancelar NF"
                        @click="vm.abrirCancelar(row)"
                    >
                        <XCircle class="size-4" />
                    </Button>
                </div>
            </template>
        </AppDataTable>

        <Dialog v-model:open="vm.cancelarVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Cancelar Nota Fiscal</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-2 pt-2">
                    <MessageBox severity="warn">
                        O cancelamento deve ocorrer em até 24h após a autorização.
                    </MessageBox>
                    <Field>
                        <FieldLabel>Motivo (mín. 15 caracteres)</FieldLabel>
                        <Textarea v-model="vm.motivo" rows="3" />
                    </Field>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.cancelarVisible = false">Voltar</Button>
                    <Button variant="destructive" :disabled="vm.motivo.length < 15" @click="vm.cancelar">
                        <X class="size-4" />
                        Cancelar NF
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
