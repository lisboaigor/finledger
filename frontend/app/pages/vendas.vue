<script setup lang="ts">
import { Box, Check, CreditCard, Eye, LoaderCircle, Pencil, Plus, Printer, RotateCcw, Search, Trash2, Undo2, User } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogScrollContent,
  DialogTitle,
} from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { Textarea } from '@/components/ui/textarea'
import { listarLixeiraVendas, restaurarVenda } from '~/models/vendas'

const vm = useVendasViewModel()
const { formatCentavos } = useFormat()
const { isAdmin } = useAuth()
const { apiFetch } = useApi()

const lixeira = useLixeira({
    listar: async () => (await listarLixeiraVendas(apiFetch)).vendas,
    restaurar: (id) => restaurarVenda(apiFetch, id),
    idDe: (v) => v.venda_id,
    aposRestaurar: () => vm.carregar(),
})

// Linhas com campos resolvidos para a busca global funcionar sobre texto visível.
const lixeiraRows = computed(() =>
    lixeira.itens.map((v) => ({
        ...v,
        cliente_nome: vm.nomeCliente(v.cliente_id),
        pagamento: v.forma_pagamento || '—',
    })),
)
const dataCurta = (iso: string) => new Date(iso).toLocaleDateString('pt-BR')

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Vendas</h1>
            <p class="text-muted-foreground">Ponto de venda e histórico.</p>
        </div>

        <div class="mb-4 flex justify-between">
            <InputGroup class="mr-2 flex-1 max-w-xl">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput
                    v-model="vm.busca"
                    placeholder="Buscar por cliente, status, pagamento ou produto vendido"
                    @update:model-value="vm.onBuscaChange"
                />
            </InputGroup>
            <div class="ml-2 flex shrink-0 gap-2">
                <Button v-if="isAdmin" variant="outline" title="Vendas arquivadas pela limpeza automática" @click="lixeira.abrir">
                    <Trash2 class="size-4" />
                    Lixeira
                </Button>
                <Button @click="vm.novaVisible = true">
                    <Plus class="size-4" />
                    Nova Venda
                </Button>
            </div>
        </div>

        <AppDataTable
            :rows="vm.vendasFiltradas"
            :loading="vm.loading"
            row-key="venda_id"
            empty-text="Nenhuma venda registrada."
            :columns="[
                { key: 'cliente', label: 'Cliente' },
                { key: 'total', label: 'Total' },
                { key: 'pagamento', label: 'Pagamento' },
                { key: 'status', label: 'Status' },
                { key: 'acoes', label: 'Ações', class: 'w-32' },
            ]"
        >
            <template #cell-cliente="{ row }">{{ vm.nomeCliente(row.cliente_id) }}</template>
            <template #cell-total="{ row }">{{ formatCentavos(row.total_centavos) }}</template>
            <template #cell-pagamento="{ row }">{{ row.forma_pagamento || '—' }}</template>
            <template #cell-status="{ row }">
                <StatusBadge :value="row.status" :severity="vm.statusSeverity(row.status)" />
            </template>
            <template #cell-acoes="{ row }">
                <div class="flex gap-1">
                    <Button variant="ghost" size="icon-sm" @click="vm.abrirDetalhe(row.venda_id)">
                        <Eye class="size-4" />
                    </Button>
                    <Button
                        v-if="vm.podeEditarOuExcluir(row)"
                        variant="ghost"
                        size="icon-sm"
                        title="Editar"
                        @click="vm.abrirDetalhe(row.venda_id)"
                    >
                        <Pencil class="size-4" />
                    </Button>
                    <Button
                        v-if="vm.podeEditarOuExcluir(row)"
                        variant="ghost"
                        size="icon-sm"
                        class="text-destructive"
                        title="Excluir"
                        @click="vm.abrirExclusao(row)"
                    >
                        <Trash2 class="size-4" />
                    </Button>
                </div>
            </template>
        </AppDataTable>

        <!-- Lixeira: vendas arquivadas pela rotina de limpeza (nada é excluído) -->
        <Dialog v-model:open="lixeira.visible">
            <DialogScrollContent class="sm:max-w-3xl">
                <DialogHeader>
                    <DialogTitle>Lixeira de vendas</DialogTitle>
                </DialogHeader>
                <p class="mb-3 text-sm text-muted-foreground">
                    Vendas abandonadas ou canceladas, arquivadas automaticamente após o prazo definido em
                    Configurações. Nada foi excluído — restaure para voltar à listagem.
                </p>
                <AppDataTable
                    :rows="lixeiraRows"
                    :loading="lixeira.loading"
                    row-key="venda_id"
                    empty-text="Lixeira vazia."
                    search-placeholder="Buscar por cliente, status ou pagamento"
                    :search-fields="['cliente_nome', 'status', 'pagamento']"
                    initial-sort-key="arquivada_em"
                    initial-sort-desc
                    :columns="[
                        { key: 'cliente_nome', label: 'Cliente', sortable: true },
                        { key: 'total_centavos', label: 'Total', sortable: true },
                        { key: 'pagamento', label: 'Pagamento', sortable: true },
                        { key: 'status', label: 'Status', sortable: true },
                        { key: 'criada_em', label: 'Criada em', sortable: true },
                        { key: 'arquivada_em', label: 'Arquivada em', sortable: true },
                        { key: 'acoes', label: '', class: 'w-32' },
                    ]"
                >
                    <template #cell-total_centavos="{ row }">{{ formatCentavos(row.total_centavos) }}</template>
                    <template #cell-status="{ row }">
                        <StatusBadge :value="row.status" :severity="vm.statusSeverity(row.status)" />
                    </template>
                    <template #cell-criada_em="{ row }">{{ dataCurta(row.criada_em) }}</template>
                    <template #cell-arquivada_em="{ row }">{{ dataCurta(row.arquivada_em) }}</template>
                    <template #cell-acoes="{ row }">
                        <Button
                            variant="outline"
                            size="sm"
                            :disabled="lixeira.restaurando === row.venda_id"
                            @click="lixeira.restaurar(row)"
                        >
                            <LoaderCircle v-if="lixeira.restaurando === row.venda_id" class="size-4 animate-spin" />
                            <Undo2 v-else class="size-4" />
                            Restaurar
                        </Button>
                    </template>
                </AppDataTable>
            </DialogScrollContent>
        </Dialog>

        <!-- Nova venda -->
        <Dialog v-model:open="vm.novaVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Nova Venda</DialogTitle>
                </DialogHeader>
                <Field>
                    <FieldLabel>Cliente (opcional)</FieldLabel>
                    <SearchSelect
                        v-model="vm.novoCliente"
                        :options="vm.opcoesCliente"
                        placeholder="Consumidor final"
                        clearable
                    />
                </Field>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.novaVisible = false">Cancelar</Button>
                    <Button @click="vm.iniciarVenda">
                        <Check class="size-4" />
                        Iniciar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <!-- Detalhe -->
        <Dialog v-model:open="vm.detalheVisible">
            <DialogScrollContent class="sm:max-w-5xl">
                <DialogHeader>
                    <DialogTitle>Detalhe da Venda</DialogTitle>
                </DialogHeader>

                <div v-if="vm.carregandoDetalhe" class="py-8 text-center">
                    <LoaderCircle class="mx-auto size-8 animate-spin text-muted-foreground" />
                </div>
                <div v-else-if="vm.detalhe" class="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_20rem] lg:items-start">
                    <!-- Coluna principal: itens -->
                    <div class="flex min-w-0 flex-col gap-4">
                        <!-- Adicionar item (somente em andamento) -->
                        <AppFieldset v-if="vm.emAndamento" legend="Adicionar item">
                            <template #legend>
                                <span class="flex items-center gap-2">
                                    <Plus class="size-4" />
                                    <span>Adicionar item</span>
                                </span>
                            </template>

                            <div class="flex flex-col gap-3 sm:flex-row sm:items-end">
                                <Field class="min-w-0 flex-1">
                                    <FieldLabel>Produto</FieldLabel>
                                    <SearchSelect
                                        v-model="vm.novoItem.produto_id"
                                        :options="vm.opcoesProduto"
                                        placeholder="Selecione"
                                    />
                                </Field>
                                <Field class="sm:w-32">
                                    <FieldLabel>Qtd.</FieldLabel>
                                    <Input v-model.number="vm.novoItem.quantidade" type="number" min="1" />
                                </Field>
                                <Button
                                    class="w-full shrink-0 sm:w-auto"
                                    :disabled="!vm.novoItem.produto_id || (vm.estoqueInsuficiente && !vm.novoItem.vender_sem_estoque)"
                                    @click="vm.adicionarItem"
                                >
                                    <Plus class="size-4" />
                                    Adicionar
                                </Button>
                            </div>
                            <MessageBox v-if="vm.estoqueInsuficiente" severity="warn" class="mt-3">
                                Estoque insuficiente para esta quantidade.
                            </MessageBox>
                            <div v-if="vm.estoqueInsuficiente" class="mt-3 flex items-center gap-2">
                                <Checkbox id="vender-sem-estoque" v-model="vm.novoItem.vender_sem_estoque" />
                                <label for="vender-sem-estoque" class="cursor-pointer text-sm">Vender sob encomenda</label>
                            </div>
                        </AppFieldset>

                        <AppFieldset legend="Itens" class="min-w-0">
                            <template #legend>
                                <span class="flex items-center gap-2">
                                    <Box class="size-4" />
                                    <span>Itens</span>
                                </span>
                            </template>

                            <AppDataTable
                                :rows="vm.detalhe.itens"
                                row-key="item_id"
                                empty-text="Nenhum item adicionado."
                                :columns="[
                                    { key: 'sku', label: 'SKU' },
                                    { key: 'descricao', label: 'Descrição' },
                                    { key: 'quantidade', label: 'Qtd.' },
                                    { key: 'preco', label: 'Preço unit.' },
                                    { key: 'subtotal', label: 'Subtotal' },
                                    ...(vm.emAndamento ? [{ key: 'acoes', label: '', class: 'w-12' }] : []),
                                ]"
                                :page-size-options="[]"
                            >
                                <template #cell-preco="{ row }">{{ formatCentavos(row.preco_unitario_centavos) }}</template>
                                <template #cell-subtotal="{ row }">
                                    {{ formatCentavos(row.preco_unitario_centavos * row.quantidade) }}
                                </template>
                                <template #cell-acoes="{ row }">
                                    <Button variant="ghost" size="icon-sm" class="text-destructive" @click="vm.removerItem(row)">
                                        <Trash2 class="size-4" />
                                    </Button>
                                </template>
                            </AppDataTable>
                        </AppFieldset>
                    </div>

                    <!-- Coluna lateral: resumo, cliente, pagamento -->
                    <div class="flex min-w-0 flex-col gap-4">
                        <!-- Resumo -->
                        <div class="rounded-lg border bg-muted/40 p-4">
                            <div class="flex items-start justify-between gap-4">
                                <StatusBadge :value="vm.detalhe.venda.status" :severity="vm.statusSeverity(vm.detalhe.venda.status)" />
                                <div class="text-right">
                                    <div class="text-sm text-muted-foreground">Total</div>
                                    <div class="text-xl font-semibold">{{ formatCentavos(vm.detalhe.venda.total_centavos) }}</div>
                                </div>
                            </div>
                            <p v-if="!vm.emAndamento" class="mt-3 text-sm">
                                <strong>Cliente:</strong> {{ vm.nomeCliente(vm.detalhe.venda.cliente_id) }}
                            </p>
                        </div>

                        <!-- Edição de cliente (somente em andamento) -->
                        <AppFieldset v-if="vm.emAndamento" legend="Cliente">
                            <template #legend>
                                <span class="flex items-center gap-2">
                                    <User class="size-4" />
                                    <span>Cliente</span>
                                </span>
                            </template>

                            <div class="flex flex-col gap-2">
                                <SearchSelect
                                    v-model="vm.editandoCliente"
                                    :options="vm.opcoesCliente"
                                    placeholder="Consumidor final"
                                    clearable
                                />
                                <Button variant="secondary" class="w-full" :disabled="vm.salvandoCliente" @click="vm.salvarCliente">
                                    <LoaderCircle v-if="vm.salvandoCliente" class="size-4 animate-spin" />
                                    <Check v-else class="size-4" />
                                    Salvar cliente
                                </Button>
                            </div>
                        </AppFieldset>

                        <!-- Forma de pagamento (somente em andamento) -->
                        <AppFieldset v-if="vm.emAndamento" legend="Forma de pagamento">
                            <template #legend>
                                <span class="flex items-center gap-2">
                                    <CreditCard class="size-4" />
                                    <span>Forma de pagamento</span>
                                </span>
                            </template>

                            <div class="flex flex-col gap-3">
                                <Field>
                                    <FieldLabel>Forma de pagamento</FieldLabel>
                                    <SearchSelect v-model="vm.formaTipo" :options="vm.formasOpcoes" placeholder="Selecione" />
                                </Field>

                                <Field v-if="vm.formaTipo === 'CartaoCredito'">
                                    <FieldLabel>Parcelas</FieldLabel>
                                    <Input v-model.number="vm.parcelas" type="number" min="1" max="12" />
                                </Field>
                                <Field v-if="vm.formaTipo === 'Prazo'">
                                    <FieldLabel>Dias</FieldLabel>
                                    <Input v-model.number="vm.prazoDias" type="number" min="1" />
                                </Field>

                                <Button variant="secondary" class="w-full" @click="vm.definirPagamento">
                                    <CreditCard class="size-4" />
                                    Definir forma de pagamento
                                </Button>
                            </div>
                        </AppFieldset>
                    </div>
                </div>

                <DialogFooter>
                    <template v-if="vm.detalhe && vm.emAndamento">
                        <Button variant="ghost" class="text-destructive" @click="vm.abrirExclusao(vm.detalhe.venda)">
                            <Trash2 class="size-4" />
                            Excluir Venda
                        </Button>
                        <Button :disabled="!vm.detalhe.itens.length" @click="vm.confirmarVenda">
                            <Check class="size-4" />
                            Confirmar Venda
                        </Button>
                    </template>
                    <template v-else>
                        <Button
                            v-if="vm.vendaConfirmada && vm.detalhe?.itens.length"
                            variant="outline"
                            class="text-amber-600"
                            @click="vm.abrirDevolucao"
                        >
                            <RotateCcw class="size-4" />
                            Devolver Itens
                        </Button>
                        <Button v-if="vm.detalhe?.itens.length" variant="outline" @click="vm.imprimirVenda">
                            <Printer class="size-4" />
                            Imprimir Recibo
                        </Button>
                        <Button variant="ghost" @click="vm.detalheVisible = false">Fechar</Button>
                    </template>
                </DialogFooter>
            </DialogScrollContent>
        </Dialog>

        <!-- Devolução de itens -->
        <Dialog v-model:open="vm.devolverVisible">
            <DialogContent class="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>Devolver Itens</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-4">
                    <MessageBox severity="info">
                        Os itens devolvidos voltam ao estoque. Devolver tudo desfaz a venda; devolução
                        parcial mantém a venda com os itens restantes (a nota fiscal é cancelada e
                        reemitida quando a integração com a SEFAZ estiver ativa).
                    </MessageBox>
                    <div v-for="item in vm.itensDevolucao" :key="item.item_id" class="flex items-center gap-3">
                        <span class="min-w-0 flex-1 text-sm">{{ item.sku }} — {{ item.descricao }}</span>
                        <span class="shrink-0 text-xs text-muted-foreground">vendidas: {{ item.vendida }}</span>
                        <InputQuantity v-model="item.quantidade" :min="0" :max="item.vendida" class="shrink-0" />
                    </div>
                    <Field>
                        <FieldLabel>Motivo da devolução</FieldLabel>
                        <Textarea v-model="vm.motivoDevolucao" rows="2" />
                    </Field>
                    <MessageBox v-if="vm.devolucaoTotal" severity="warn">
                        Devolução total: a venda será desfeita e a conta a receber em aberto, estornada.
                    </MessageBox>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.devolverVisible = false">Voltar</Button>
                    <Button
                        variant="outline"
                        class="text-amber-600"
                        :disabled="!vm.devolucaoValida"
                        @click="vm.confirmarDevolucao"
                    >
                        <LoaderCircle v-if="vm.salvandoDevolucao" class="size-4 animate-spin" />
                        <RotateCcw v-else class="size-4" />
                        {{ vm.devolucaoTotal ? 'Devolver Tudo e Desfazer Venda' : 'Registrar Devolução' }}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <Dialog v-model:open="vm.cancelarVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Excluir Venda</DialogTitle>
                </DialogHeader>
                <Field>
                    <FieldLabel>Motivo da exclusão</FieldLabel>
                    <Textarea v-model="vm.motivoCancelamento" rows="3" />
                </Field>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.cancelarVisible = false">Voltar</Button>
                    <Button variant="destructive" :disabled="!vm.motivoCancelamento" @click="vm.cancelarVenda">
                        <Trash2 class="size-4" />
                        Excluir Venda
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
