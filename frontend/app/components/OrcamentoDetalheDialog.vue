<script setup lang="ts">
import { Box, Check, FileEdit, LoaderCircle, Percent, Plus, Printer, Send, Trash2, Wallet, X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogFooter, DialogHeader, DialogScrollContent, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

const props = defineProps<{
    vm: ReturnType<typeof useOrcamentosViewModel>
}>()

const visible = defineModel<boolean>('visible', { required: true })

const { formatCentavos } = useFormat()
const { statusSeverity } = useOrcamentoStatus()

const detalhe = computed(() => props.vm.detalhe)
</script>

<template>
    <Dialog v-model:open="visible">
        <DialogScrollContent class="sm:max-w-5xl">
            <DialogHeader>
                <DialogTitle>Detalhe do Orçamento</DialogTitle>
            </DialogHeader>

            <div v-if="vm.carregandoDetalhe" class="py-8 text-center">
                <LoaderCircle class="mx-auto size-8 animate-spin text-muted-foreground" />
            </div>

            <div v-else-if="detalhe" class="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_20rem] lg:items-start">
                <!-- Coluna principal: itens -->
                <AppFieldset legend="Itens" class="min-w-0">
                    <template #legend>
                        <span class="flex items-center gap-2">
                            <Box class="size-4" />
                            <span>Itens</span>
                        </span>
                    </template>

                    <!-- Adicionar item: horizontal -->
                    <div v-if="vm.isRascunho" class="mb-4 flex flex-col gap-2 sm:flex-row sm:items-end">
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
                            :disabled="!vm.novoItem.produto_id"
                            @click="vm.adicionarItem"
                        >
                            <Plus class="size-4" />
                            Adicionar
                        </Button>
                    </div>

                    <AppDataTable
                        :rows="detalhe.itens"
                        row-key="item_id"
                        empty-text="Nenhum item."
                        :page-size-options="[]"
                        :columns="[
                            { key: 'sku', label: 'SKU' },
                            { key: 'descricao', label: 'Descrição' },
                            { key: 'quantidade', label: 'Qtd.' },
                            { key: 'preco', label: 'Preço unit.' },
                            { key: 'subtotal', label: 'Subtotal' },
                            ...(vm.isRascunho ? [{ key: 'acoes', label: '', class: 'w-12' }] : []),
                        ]"
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

                <!-- Coluna lateral: resumo, dados, desconto/totais -->
                <div class="flex min-w-0 flex-col gap-4">
                    <!-- 1. Resumo -->
                    <div class="rounded-lg border bg-muted/40 p-4">
                        <div class="flex items-start justify-between gap-4">
                            <StatusBadge :value="detalhe.orcamento.status" :severity="statusSeverity(detalhe.orcamento.status)" />
                            <div class="text-right">
                                <div class="text-sm text-muted-foreground">Total</div>
                                <div class="text-xl font-semibold">
                                    {{ formatCentavos(detalhe.orcamento.total_centavos) }}
                                </div>
                            </div>
                        </div>

                        <div class="mt-3 text-sm">
                            <div v-if="!vm.isRascunho">
                                <strong>Cliente:</strong>
                                {{ vm.nomeCliente(detalhe.orcamento) }}
                            </div>
                            <div class="text-muted-foreground">
                                {{ detalhe.itens.length }}
                                {{ detalhe.itens.length === 1 ? 'item' : 'itens' }}
                            </div>
                        </div>
                    </div>

                    <!-- 2. Dados do orçamento -->
                    <AppFieldset v-if="vm.isRascunho" legend="Dados do orçamento">
                        <template #legend>
                            <span class="flex items-center gap-2">
                                <FileEdit class="size-4" />
                                <span>Dados do orçamento</span>
                            </span>
                        </template>

                        <div class="flex flex-col gap-3">
                            <Field>
                                <FieldLabel>Cliente</FieldLabel>
                                <SearchSelect
                                    v-model="vm.editCliente"
                                    :options="vm.opcoesCliente"
                                    placeholder="Sem cliente"
                                    clearable
                                />
                            </Field>
                            <Field>
                                <FieldLabel>Ou nome do cliente de balcão</FieldLabel>
                                <Input
                                    v-model="vm.editClienteAvulso"
                                    placeholder="Ex.: João, cliente de balcão"
                                    :disabled="!!vm.editCliente"
                                    maxlength="120"
                                />
                            </Field>
                            <Field>
                                <FieldLabel>Validade (dias)</FieldLabel>
                                <Input v-model.number="vm.editValidade" type="number" min="1" />
                            </Field>
                            <Button
                                variant="secondary"
                                class="w-full"
                                :disabled="vm.salvandoEdicao"
                                @click="vm.salvarEdicao"
                            >
                                <LoaderCircle v-if="vm.salvandoEdicao" class="size-4 animate-spin" />
                                <Check v-else class="size-4" />
                                Salvar dados
                            </Button>
                        </div>
                    </AppFieldset>
                    <p v-else class="text-sm">
                        <strong>Cliente:</strong>
                        {{ vm.nomeCliente(detalhe.orcamento) }}
                    </p>

                    <!-- 3. Desconto -->
                    <AppFieldset v-if="vm.isRascunho" legend="Desconto">
                        <template #legend>
                            <span class="flex items-center gap-2">
                                <Wallet class="size-4" />
                                <span>Desconto</span>
                            </span>
                        </template>

                        <div class="flex flex-col gap-2">
                            <!-- InputMoney já sincroniza o v-model no blur; sincronizarDesconto
                                 mantém os dados derivados (percentual) em dia a cada alteração. -->
                            <InputMoney
                                :model-value="vm.descontoValor"
                                @update:model-value="vm.sincronizarDesconto"
                            />
                            <Button variant="secondary" class="w-full" @click="vm.aplicarDesconto">
                                <Percent class="size-4" />
                                Aplicar
                            </Button>
                        </div>
                    </AppFieldset>

                    <!-- 4. Totais -->
                    <div class="rounded-lg border p-4">
                        <div class="flex flex-col gap-1">
                            <div class="flex justify-between text-sm">
                                <span class="text-muted-foreground">Subtotal</span>
                                <span>{{ formatCentavos(vm.subtotalCentavos) }}</span>
                            </div>
                            <div class="flex justify-between text-sm">
                                <span class="text-muted-foreground">Desconto</span>
                                <span>-{{ formatCentavos(detalhe.orcamento.desconto_centavos) }}</span>
                            </div>
                            <hr class="my-1 border-border">
                            <div class="flex justify-between font-semibold">
                                <span>Total</span>
                                <span>{{ formatCentavos(detalhe.orcamento.total_centavos) }}</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <DialogFooter class="sm:justify-between">
                <div class="flex gap-1">
                    <Button v-if="detalhe" variant="ghost" @click="vm.imprimirOrcamento">
                        <Printer class="size-4" />
                        Imprimir
                    </Button>
                    <template v-if="detalhe && (vm.isRascunho || vm.isEmitido)">
                        <Button variant="ghost" class="text-destructive" @click="vm.abrirExclusao(detalhe.orcamento)">
                            <Trash2 class="size-4" />
                            Excluir
                        </Button>
                    </template>
                </div>

                <div class="flex gap-2">
                    <template v-if="detalhe && vm.isRascunho">
                        <Button variant="ghost" @click="visible = false">Fechar</Button>
                        <Button :disabled="!detalhe.itens.length" @click="vm.emitir">
                            <Send class="size-4" />
                            Emitir
                        </Button>
                    </template>
                    <template v-else-if="detalhe && vm.isEmitido">
                        <Button variant="ghost" class="text-destructive" @click="vm.recusarVisible = true">
                            <X class="size-4" />
                            Recusar
                        </Button>
                        <Button class="bg-emerald-600 text-white hover:bg-emerald-600/90" @click="vm.aceitar">
                            <Check class="size-4" />
                            Aceitar
                        </Button>
                    </template>
                    <Button v-else-if="detalhe" variant="ghost" @click="visible = false">Fechar</Button>
                </div>
            </DialogFooter>
        </DialogScrollContent>
    </Dialog>
</template>
