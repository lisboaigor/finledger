<script setup lang="ts">
import { Check, Flag, LoaderCircle, Plus, Printer, Search, SlidersHorizontal } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { Textarea } from '@/components/ui/textarea'

const vm = useEstoqueViewModel()
const { formatCentavos } = useFormat()

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4">
        <div class="mb-4">
            <h1 class="text-2xl font-semibold">Estoque</h1>
            <p class="text-muted-foreground">Saldos, entradas e ajustes.</p>
        </div>

        <div class="mb-4 flex justify-between items-center">
            <InputGroup class="mr-2 flex-1 max-w-xl">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput v-model="vm.busca" placeholder="Buscar por SKU, descrição, marca ou categoria" />
            </InputGroup>
            <div class="flex gap-2">
                <Button variant="outline" class="mr-2" @click="vm.imprimirInventario">
                    <Printer class="size-4" />
                    Imprimir Inventário
                </Button>
                <Button @click="vm.abrirEntrada">
                    <Plus class="size-4" />
                    Registrar Entrada
                </Button>
            </div>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-4">
            <Card class="mb-4">
                <CardContent>
                    <div class="text-sm text-muted-foreground mb-1">Valor do estoque (a custo)</div>
                    <div class="text-2xl font-semibold">{{ formatCentavos(vm.valorCustoCents) }}</div>
                </CardContent>
            </Card>
            <Card class="mb-4">
                <CardContent>
                    <div class="text-sm text-muted-foreground mb-1">Itens com saldo</div>
                    <div class="text-2xl font-semibold">{{ vm.itensComSaldo }}</div>
                </CardContent>
            </Card>
            <Card class="mb-4">
                <CardContent>
                    <div class="text-sm text-muted-foreground mb-1">Unidades totais</div>
                    <div class="text-2xl font-semibold">{{ vm.totalUnidades }}</div>
                </CardContent>
            </Card>
        </div>

        <AppDataTable
            :rows="vm.linhasFiltradas"
            :loading="vm.loading"
            row-key="produto_id"
            empty-text="Nenhum saldo de estoque."
            :columns="[
                { key: 'sku', label: 'SKU', sortable: true },
                { key: 'descricao', label: 'Produto', sortable: true },
                { key: 'quantidade', label: 'Quantidade', sortable: true },
                { key: 'custo_medio', label: 'Custo médio' },
                { key: 'valor_custo', label: 'Valor a custo' },
                { key: 'estoque_minimo', label: 'Mínimo', sortable: true },
                { key: 'acoes', label: 'Ações', class: 'w-32' },
            ]"
        >
            <template #cell-quantidade="{ row }">
                <StatusBadge :value="String(row.quantidade)" :severity="row.quantidade > 0 ? 'success' : 'warn'" />
            </template>
            <template #cell-custo_medio="{ row }">{{ formatCentavos(row.custo_medio) }}</template>
            <template #cell-valor_custo="{ row }">{{ formatCentavos(row.quantidade * row.custo_medio) }}</template>
            <template #cell-estoque_minimo="{ row }">
                <StatusBadge
                    :value="String(row.estoque_minimo)"
                    :severity="row.estoque_minimo > 0 && row.quantidade <= row.estoque_minimo ? 'danger' : 'secondary'"
                />
            </template>
            <template #cell-acoes="{ row }">
                <div class="flex gap-1">
                    <Button variant="ghost" size="icon-sm" title="Ajustar saldo" @click="vm.abrirAjuste(row)">
                        <SlidersHorizontal class="size-4" />
                    </Button>
                    <Button variant="ghost" size="icon-sm" title="Definir estoque mínimo" @click="vm.abrirMinimo(row)">
                        <Flag class="size-4" />
                    </Button>
                </div>
            </template>
        </AppDataTable>

        <Dialog v-model:open="vm.entradaVisible">
            <DialogContent class="sm:max-w-5xl">
                <DialogHeader>
                    <DialogTitle>Registrar Entrada</DialogTitle>
                </DialogHeader>
                <div class="grid max-h-[70vh] grid-cols-1 gap-4 overflow-y-auto pt-2 pr-1 lg:grid-cols-2 lg:items-start">
                    <!-- Coluna esquerda: dados da entrada -->
                    <div class="flex min-w-0 flex-col gap-4">
                        <Field>
                            <FieldLabel>Produto</FieldLabel>
                            <SearchSelect v-model="vm.entrada.produto_id" :options="vm.opcoesProduto" placeholder="Selecione o produto" />
                        </Field>
                        <Field>
                            <FieldLabel>Fornecedor (opcional)</FieldLabel>
                            <SearchSelect
                                v-model="vm.entrada.fornecedor_id"
                                :options="vm.opcoesFornecedor"
                                placeholder="Sem fornecedor"
                                clearable
                            />
                        </Field>
                        <div class="grid grid-cols-2 gap-3">
                            <Field>
                                <FieldLabel>Quantidade</FieldLabel>
                                <InputQuantity v-model="vm.entrada.quantidade" :min="1" />
                            </Field>
                            <Field>
                                <FieldLabel>Custo unitário</FieldLabel>
                                <InputMoney v-model="vm.entrada.custo_unitario" />
                            </Field>
                        </div>
                        <Field>
                            <FieldLabel>Frete da remessa (opcional)</FieldLabel>
                            <InputMoney v-model="vm.entrada.frete_remessa" />
                            <p class="text-xs text-muted-foreground">
                                Dividido pelas unidades e somado ao custo que entra no estoque
                                <template v-if="vm.entrada.frete_remessa > 0 && vm.entrada.quantidade > 0">
                                    — custo final: {{ formatCentavos(vm.custoEfetivoUnitarioCentavos) }} por unidade
                                </template>
                            </p>
                        </Field>
                        <div class="grid grid-cols-2 gap-3">
                            <Field>
                                <FieldLabel>Motivo</FieldLabel>
                                <Input v-model="vm.entrada.motivo" placeholder="Ex.: Compra, devolução…" />
                            </Field>
                            <Field>
                                <FieldLabel>Nota fiscal (nº)</FieldLabel>
                                <Input v-model="vm.entrada.nota_fiscal" placeholder="NF do fornecedor" />
                            </Field>
                        </div>
                    </div>

                    <!-- Coluna direita: apoio à decisão de preço -->
                    <div class="flex min-w-0 flex-col gap-4">
                        <PainelPrecificacao
                            v-if="vm.produtoDaEntrada && vm.entrada.custo_unitario > 0"
                            :custo-centavos="vm.custoEfetivoUnitarioCentavos"
                            :categoria="vm.produtoDaEntrada.categoria"
                            :produto-id="vm.produtoDaEntrada.produto_id"
                            :preco-digitado-centavos="
                                vm.atualizarPrecoVenda && vm.novoPrecoVenda != null
                                    ? Math.round(vm.novoPrecoVenda * 100)
                                    : vm.produtoDaEntrada.preco_venda
                            "
                            :preco-vigente-centavos="vm.produtoDaEntrada.preco_venda"
                            :sku="vm.produtoDaEntrada.sku"
                            :descricao="vm.produtoDaEntrada.descricao"
                            @usar-sugestao="
                                (centavos) => {
                                    vm.atualizarPrecoVenda = true
                                    vm.novoPrecoVenda = centavos / 100
                                }
                            "
                        />

                        <!-- Atualização opcional do preço do produto (só admin) -->
                        <template v-if="vm.isAdmin && vm.produtoDaEntrada">
                            <div class="flex items-center gap-2">
                                <Checkbox id="atualizar-preco" v-model="vm.atualizarPrecoVenda" />
                                <label for="atualizar-preco" class="text-sm cursor-pointer">
                                    Atualizar preço de custo/venda do produto com esta compra
                                </label>
                            </div>
                            <Field v-if="vm.atualizarPrecoVenda">
                                <FieldLabel>Novo preço de venda</FieldLabel>
                                <InputMoney v-model="vm.novoPrecoVenda" />
                                <p class="text-xs text-muted-foreground">
                                    Você decide o valor — a sugestão é só um ponto de partida.
                                </p>
                            </Field>
                        </template>
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.entradaVisible = false">Cancelar</Button>
                    <Button :disabled="vm.salvandoEntrada || !vm.entrada.produto_id" @click="vm.registrarEntrada">
                        <LoaderCircle v-if="vm.salvandoEntrada" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Registrar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>

        <Dialog v-model:open="vm.ajusteVisible">
            <DialogContent class="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Ajustar Saldo</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-4 pt-2">
                    <Field>
                        <FieldLabel>Nova quantidade</FieldLabel>
                        <InputQuantity v-model="vm.ajuste.quantidade_nova" :min="0" />
                    </Field>
                    <Field v-if="vm.ajusteAumenta">
                        <FieldLabel>Custo unitário das unidades acrescentadas</FieldLabel>
                        <InputMoney v-model="vm.ajuste.custo_unitario" />
                        <p class="text-xs text-muted-foreground">
                            Usado para recalcular o custo médio. Sugerido: o custo médio atual.
                        </p>
                    </Field>
                    <Field>
                        <FieldLabel>Justificativa</FieldLabel>
                        <Textarea v-model="vm.ajuste.justificativa" rows="3" />
                    </Field>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.ajusteVisible = false">Cancelar</Button>
                    <Button
                        :disabled="vm.salvandoAjuste || !vm.ajuste.justificativa || (vm.ajusteAumenta && vm.ajuste.custo_unitario <= 0)"
                        @click="vm.registrarAjuste"
                    >
                        <LoaderCircle v-if="vm.salvandoAjuste" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Ajustar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
        <Dialog v-model:open="vm.minimoVisible">
            <DialogContent class="sm:max-w-sm">
                <DialogHeader>
                    <DialogTitle>Estoque Mínimo</DialogTitle>
                </DialogHeader>
                <div class="flex flex-col gap-2 pt-2">
                    <Field>
                        <FieldLabel>Quantidade mínima em estoque</FieldLabel>
                        <InputQuantity v-model="vm.minimo.estoque_minimo" :min="0" />
                    </Field>
                    <p class="text-xs text-muted-foreground">
                        Ao atingir esse nível, o sistema sinaliza reposição.
                    </p>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.minimoVisible = false">Cancelar</Button>
                    <Button :disabled="vm.salvandoMinimo" @click="vm.salvarMinimo">
                        <LoaderCircle v-if="vm.salvandoMinimo" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Salvar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
