<script setup lang="ts">
import { Check, Eye, LoaderCircle, Package, Plus, Printer, Search, Send, Trash2, X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogScrollContent, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { Textarea } from '@/components/ui/textarea'

const vm = useComprasViewModel()
const { formatCentavos } = useFormat()
const route = useRoute()

// Filtro global equivalente ao `filters.global` do PrimeVue DataTable, aplicado
// sobre os campos visíveis na tabela (fornecedor e status).
const pedidosFiltrados = computed(() => {
    const termo = (vm.filters.global.value || '').trim().toLowerCase()
    if (!termo) return vm.pedidos
    return vm.pedidos.filter((p) => {
        const fornecedor = vm.nomeFornecedor(p.fornecedor_id).toLowerCase()
        return fornecedor.includes(termo) || p.status.toLowerCase().includes(termo)
    })
})

onMounted(async () => {
    await vm.carregar()
    // Pedido 1-clique vindo do BI (alerta de ruptura / aba Estoque & Compras).
    const produto = route.query.produto
    if (typeof produto === 'string' && produto) {
        vm.abrirGerarPrefill(produto, Number(route.query.quantidade) || 1)
    }
})
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Compras</h1>
            <p class="text-muted-foreground">Pedidos de compra a fornecedores.</p>
        </div>
        <div class="mb-4 flex justify-between items-center">
            <InputGroup class="mr-2 flex-1 max-w-xl">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput v-model="vm.filters.global.value" placeholder="Buscar pedido..." />
            </InputGroup>
            <Button @click="vm.abrirGerar">
                <Plus class="size-4" />
                Novo Pedido
            </Button>
        </div>

        <AppDataTable
            :rows="pedidosFiltrados"
            :loading="vm.loading"
            row-key="pedido_id"
            empty-text="Nenhum pedido de compra."
            :columns="[
                { key: 'fornecedor', label: 'Fornecedor' },
                { key: 'total', label: 'Total' },
                { key: 'prazo', label: 'Prazo' },
                { key: 'status', label: 'Status' },
                { key: 'acoes', label: 'Ações', class: 'w-24' },
            ]"
        >
            <template #cell-fornecedor="{ row }">{{ vm.nomeFornecedor(row.fornecedor_id) }}</template>
            <template #cell-total="{ row }">{{ formatCentavos(row.total_centavos) }}</template>
            <template #cell-prazo="{ row }">{{ row.prazo_pagamento_dias }} dias</template>
            <template #cell-status="{ row }">
                <StatusBadge :value="row.status" :severity="vm.statusSeverity(row.status)" />
            </template>
            <template #cell-acoes="{ row }">
                <Button variant="ghost" size="icon-sm" @click="vm.abrirDetalhe(row.pedido_id)">
                    <Eye class="size-4" />
                </Button>
            </template>
        </AppDataTable>

        <!-- Gerar pedido -->
        <Dialog v-model:open="vm.gerarVisible">
            <DialogContent class="sm:max-w-2xl">
                <DialogHeader>
                    <DialogTitle>Novo Pedido de Compra</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-4">
                    <div class="grid grid-cols-2 gap-3">
                        <Field>
                            <FieldLabel>Fornecedor</FieldLabel>
                            <SearchSelect v-model="vm.gerar.fornecedor_id" :options="vm.opcoesFornecedor" placeholder="Selecione" />
                        </Field>
                        <Field>
                            <FieldLabel>Prazo de pagamento (dias)</FieldLabel>
                            <InputQuantity v-model="vm.gerar.prazo_pagamento_dias" :min="0" />
                        </Field>
                    </div>

                    <div class="flex flex-col gap-2">
                        <div class="flex items-center justify-between">
                            <label class="text-sm font-medium">Itens</label>
                            <Button variant="ghost" size="sm" @click="vm.adicionarLinha">
                                <Plus class="size-4" />
                                Adicionar item
                            </Button>
                        </div>
                        <div v-for="(linha, i) in vm.gerar.itens" :key="i" class="flex items-end gap-2">
                            <div class="flex flex-col gap-1 flex-1 min-w-0">
                                <SearchSelect
                                    v-model="linha.produto_id"
                                    :options="vm.opcoesProduto"
                                    placeholder="Produto"
                                    @update:model-value="vm.aoSelecionarProduto(linha)"
                                />
                            </div>
                            <div class="flex flex-col gap-1 shrink-0">
                                <InputQuantity v-model="linha.quantidade" :min="1" />
                            </div>
                            <div class="flex flex-col gap-1 w-32 shrink-0">
                                <InputMoney v-model="linha.custo" />
                            </div>
                            <Button variant="ghost" size="icon-sm" class="text-destructive shrink-0" @click="vm.removerLinha(i)">
                                <Trash2 class="size-4" />
                            </Button>
                        </div>
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.gerarVisible = false">Cancelar</Button>
                    <Button :disabled="vm.salvandoGerar || !vm.gerarValido" @click="vm.submeterGerar">
                        <LoaderCircle v-if="vm.salvandoGerar" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Gerar Pedido
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Detalhe -->
        <Dialog v-model:open="vm.detalheVisible">
            <DialogScrollContent class="sm:max-w-4xl">
                <DialogHeader>
                    <DialogTitle>Detalhe do Pedido</DialogTitle>
                </DialogHeader>
                <div v-if="vm.carregandoDetalhe" class="py-8 text-center">
                    <LoaderCircle class="mx-auto size-8 animate-spin text-muted-foreground" />
                </div>
                <div v-else-if="vm.detalhe" class="flex min-w-0 flex-col gap-4">
                    <div class="flex flex-wrap items-center gap-4">
                        <span><strong>Fornecedor:</strong> {{ vm.nomeFornecedor(vm.detalhe.pedido.fornecedor_id) }}</span>
                        <StatusBadge :value="vm.detalhe.pedido.status" :severity="vm.statusSeverity(vm.detalhe.pedido.status)" />
                        <span class="ml-auto text-lg font-semibold">{{ formatCentavos(vm.detalhe.pedido.total_centavos) }}</span>
                    </div>
                    <hr class="border-border">
                    <AppDataTable
                        :rows="vm.detalhe.itens"
                        row-key="produto_id"
                        empty-text="Sem itens."
                        :page-size-options="[]"
                        :columns="[
                            { key: 'produto', label: 'Produto' },
                            { key: 'quantidade', label: 'Qtd.' },
                            { key: 'custo_unit', label: 'Custo unit.' },
                            { key: 'subtotal', label: 'Subtotal' },
                        ]"
                    >
                        <template #cell-produto="{ row }">{{ vm.descProduto(row.produto_id) }}</template>
                        <template #cell-custo_unit="{ row }">{{ formatCentavos(row.custo_unitario_centavos) }}</template>
                        <template #cell-subtotal="{ row }">{{ formatCentavos(row.custo_unitario_centavos * row.quantidade) }}</template>
                    </AppDataTable>
                </div>
                <DialogFooter>
                    <Button v-if="vm.detalhe?.itens.length" variant="outline" @click="vm.imprimirPedido">
                        <Printer class="size-4" />
                        Imprimir Pedido
                    </Button>
                    <Button
                        v-if="vm.st && !['RecebidoTotal', 'Cancelado'].includes(vm.st)"
                        variant="ghost"
                        class="text-destructive"
                        @click="vm.cancelarVisible = true"
                    >
                        <X class="size-4" />
                        Cancelar Pedido
                    </Button>
                    <Button v-if="vm.st === 'Gerado'" @click="vm.aprovar">
                        <Check class="size-4" />
                        Aprovar
                    </Button>
                    <Button v-if="vm.st === 'Aprovado'" @click="vm.enviar">
                        <Send class="size-4" />
                        Enviar
                    </Button>
                    <Button
                        v-if="vm.st === 'Enviado' || vm.st === 'RecebidoParcial'"
                        class="bg-emerald-600 text-white hover:bg-emerald-700"
                        @click="vm.abrirReceber"
                    >
                        <Package class="size-4" />
                        Receber Mercadoria
                    </Button>
                </DialogFooter>
            </DialogScrollContent>
        </Dialog>

        <!-- Receber -->
        <Dialog v-model:open="vm.receberVisible">
            <DialogContent class="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>Receber Mercadoria</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-3">
                    <div v-for="item in vm.itensReceber" :key="item.produto_id" class="flex items-center gap-3">
                        <span class="flex-1 min-w-0 truncate text-sm" :title="vm.descProduto(item.produto_id)">{{ vm.descProduto(item.produto_id) }}</span>
                        <InputQuantity v-model="item.quantidade" :min="0" class="w-32 shrink-0" />
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.receberVisible = false">Cancelar</Button>
                    <Button class="bg-emerald-600 text-white hover:bg-emerald-700" @click="vm.confirmarRecebimento">
                        <Check class="size-4" />
                        Confirmar Recebimento
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Cancelar -->
        <Dialog v-model:open="vm.cancelarVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Cancelar Pedido</DialogTitle>
                </DialogHeader>
                <Field>
                    <FieldLabel>Motivo</FieldLabel>
                    <Textarea v-model="vm.motivoCancel" rows="3" />
                </Field>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.cancelarVisible = false">Voltar</Button>
                    <Button variant="destructive" :disabled="!vm.motivoCancel" @click="vm.cancelar">
                        <X class="size-4" />
                        Cancelar Pedido
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
