<script setup lang="ts">
import { Check, DollarSign, Undo2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'

const { formatCentavos } = useFormat()
const vm = useFinanceiroViewModel()

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Financeiro</h1>
            <p class="text-muted-foreground">Contas a receber e a pagar.</p>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-6">
            <div class="rounded-lg border bg-card p-5">
                <span class="text-sm text-muted-foreground">A receber (saldo aberto)</span>
                <div class="text-2xl font-semibold text-green-600">{{ formatCentavos(vm.totalReceber) }}</div>
            </div>
            <div class="rounded-lg border bg-card p-5">
                <span class="text-sm text-muted-foreground">A pagar (saldo aberto)</span>
                <div class="text-2xl font-semibold text-red-600">{{ formatCentavos(vm.totalPagar) }}</div>
            </div>
        </div>

        <Tabs default-value="receber" class="w-full">
            <TabsList>
                <TabsTrigger value="receber">Contas a Receber</TabsTrigger>
                <TabsTrigger value="pagar">Contas a Pagar</TabsTrigger>
            </TabsList>
            <TabsContent value="receber">
                <AppDataTable
                    :rows="vm.receber"
                    :loading="vm.loading"
                    row-key="conta_id"
                    empty-text="Nenhuma conta a receber."
                    search-placeholder="Buscar conta..."
                    :search-fields="['status']"
                    :columns="[
                        { key: 'valor_original', label: 'Valor original' },
                        { key: 'valor_recebido', label: 'Recebido' },
                        { key: 'saldo', label: 'Saldo' },
                        { key: 'status', label: 'Status' },
                        { key: 'acoes', label: 'Ações', class: 'w-36' },
                    ]"
                >
                    <template #cell-valor_original="{ row }">{{ formatCentavos(row.valor_original) }}</template>
                    <template #cell-valor_recebido="{ row }">{{ formatCentavos(row.valor_recebido) }}</template>
                    <template #cell-saldo="{ row }">{{ formatCentavos(vm.saldoReceber(row)) }}</template>
                    <template #cell-status="{ row }">
                        <StatusBadge :value="row.status" :severity="vm.statusSeverity(row.status)" />
                    </template>
                    <template #cell-acoes="{ row }">
                        <div class="flex gap-1">
                            <Button
                                v-if="!['Liquidada', 'Estornada'].includes(row.status)"
                                variant="ghost"
                                size="icon-sm"
                                class="text-emerald-600"
                                title="Registrar recebimento"
                                @click="vm.abrirPagamento('receber', row)"
                            >
                                <DollarSign class="size-4" />
                            </Button>
                            <Button
                                v-if="row.status !== 'Estornada'"
                                variant="ghost"
                                size="icon-sm"
                                class="text-destructive"
                                title="Estornar"
                                @click="vm.abrirEstorno(row)"
                            >
                                <Undo2 class="size-4" />
                            </Button>
                        </div>
                    </template>
                </AppDataTable>
            </TabsContent>
            <TabsContent value="pagar">
                <AppDataTable
                    :rows="vm.pagar"
                    :loading="vm.loading"
                    row-key="conta_id"
                    empty-text="Nenhuma conta a pagar."
                    search-placeholder="Buscar conta..."
                    :search-fields="['status']"
                    :columns="[
                        { key: 'valor_original', label: 'Valor original' },
                        { key: 'valor_pago', label: 'Pago' },
                        { key: 'saldo', label: 'Saldo' },
                        { key: 'status', label: 'Status' },
                        { key: 'acoes', label: 'Ações', class: 'w-28' },
                    ]"
                >
                    <template #cell-valor_original="{ row }">{{ formatCentavos(row.valor_original) }}</template>
                    <template #cell-valor_pago="{ row }">{{ formatCentavos(row.valor_pago) }}</template>
                    <template #cell-saldo="{ row }">{{ formatCentavos(vm.saldoPagar(row)) }}</template>
                    <template #cell-status="{ row }">
                        <StatusBadge :value="row.status" :severity="vm.statusSeverity(row.status)" />
                    </template>
                    <template #cell-acoes="{ row }">
                        <Button
                            v-if="row.status !== 'Liquidada'"
                            variant="ghost"
                            size="icon-sm"
                            class="text-emerald-600"
                            title="Efetuar pagamento"
                            @click="vm.abrirPagamento('pagar', row)"
                        >
                            <DollarSign class="size-4" />
                        </Button>
                    </template>
                </AppDataTable>
            </TabsContent>
        </Tabs>

        <Dialog v-model:open="vm.pagVisible">
            <DialogContent class="sm:max-w-sm">
                <DialogHeader>
                    <DialogTitle>{{ vm.pagTipo === 'receber' ? 'Registrar Recebimento' : 'Efetuar Pagamento' }}</DialogTitle>
                </DialogHeader>
                <Field>
                    <FieldLabel>Valor</FieldLabel>
                    <InputMoney v-model="vm.pagValor" :min="0" />
                </Field>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.pagVisible = false">Cancelar</Button>
                    <Button :disabled="vm.pagValor <= 0" @click="vm.confirmarPagamento">
                        <Check class="size-4" />
                        Confirmar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <Dialog v-model:open="vm.estornoVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Estornar Conta</DialogTitle>
                </DialogHeader>
                <Field>
                    <FieldLabel>Motivo do estorno</FieldLabel>
                    <Textarea v-model="vm.motivoEstorno" rows="3" />
                </Field>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.estornoVisible = false">Voltar</Button>
                    <Button variant="destructive" :disabled="!vm.motivoEstorno" @click="vm.estornar">
                        <Undo2 class="size-4" />
                        Estornar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
