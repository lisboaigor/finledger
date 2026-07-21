<script setup lang="ts">
import { Ban, Check, ChevronDown, ChevronRight, LoaderCircle, Pencil, Plus, Printer, Search } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const vm = useCatalogoViewModel()
const { formatCentavos } = useFormat()

const ajustesAbertos = ref(false)

onMounted(vm.carregar)
</script>

<template>
    <div class="rounded-lg border bg-card p-4 h-fu">
        <div class="mb-4">
                <h1 class="text-2xl font-semibold">Catálogo de Produtos</h1>
                <p class="text-muted-foreground">Cadastro de produtos e preços.</p>
        </div>

        <div class="mb-4 flex justify-between items-center">
            <InputGroup class="mr-2 flex-1 max-w-xl">
                <InputGroupAddon>
                    <Search class="size-4 text-muted-foreground" />
                </InputGroupAddon>
                <InputGroupInput v-model="vm.filtro" placeholder="Buscar por SKU, descrição, marca ou categoria" />
            </InputGroup>

            <div class="flex gap-2">
                <Button variant="outline" @click="vm.imprimirCatalogo">
                    <Printer class="size-4" />
                    Imprimir Catálogo
                </Button>
                <Button @click="vm.abrirNovo">
                    <Plus class="size-4" />
                    Novo Produto
                </Button>
            </div>
        </div>

        <AppDataTable
            :rows="vm.produtosFiltrados"
            :loading="vm.loading"
            row-key="produto_id"
            empty-text="Nenhum produto encontrado."
            :columns="[
                { key: 'sku', label: 'SKU', sortable: true },
                { key: 'descricao', label: 'Descrição', sortable: true },
                { key: 'marca', label: 'Marca', sortable: true },
                { key: 'categoria', label: 'Categoria', sortable: true },
                { key: 'custo', label: 'Custo' },
                { key: 'venda', label: 'Venda' },
                { key: 'situacao', label: 'Situação' },
                { key: 'acoes', label: 'Ações', class: 'w-32' },
            ]"
        >
            <template #cell-marca="{ row }">{{ row.marca || '—' }}</template>
            <template #cell-categoria="{ row }">
                <div class="flex items-center gap-2">
                    <span>{{ row.categoria }}</span>
                    <StatusBadge v-if="!row.controla_estoque" value="Serviço" severity="info" />
                </div>
            </template>
            <template #cell-custo="{ row }">{{ formatCentavos(row.preco_custo) }}</template>
            <template #cell-venda="{ row }">{{ formatCentavos(row.preco_venda) }}</template>
            <template #cell-situacao="{ row }">
                <StatusBadge :value="row.ativo ? 'Ativo' : 'Inativo'" :severity="row.ativo ? 'success' : 'danger'" />
            </template>
            <template #cell-acoes="{ row }">
                <div class="flex gap-1">
                    <Button variant="ghost" size="icon-sm" @click="vm.abrirEdicao(row)">
                        <Pencil class="size-4" />
                    </Button>
                    <Button
                        variant="ghost"
                        size="icon-sm"
                        :class="row.ativo ? 'text-destructive' : 'text-emerald-600'"
                        @click="vm.alternarAtivo(row)"
                    >
                        <Ban v-if="row.ativo" class="size-4" />
                        <Check v-else class="size-4" />
                    </Button>
                </div>
            </template>
        </AppDataTable>

        <Dialog v-model:open="vm.dialogVisible">
            <DialogContent :class="vm.editando ? 'sm:max-w-7xl' : 'sm:max-w-2xl'">
                <DialogHeader>
                    <DialogTitle>{{ vm.editando ? 'Editar Produto' : 'Novo Produto' }}</DialogTitle>
                </DialogHeader>
                <div
                    class="grid h-[70vh] grid-cols-1 gap-4 pt-2 pr-1 overflow-y-auto lg:h-auto lg:max-h-[70vh] lg:items-start"
                    :class="vm.editando && 'lg:grid-cols-[40%_1fr]'"
                >
                    <!-- Coluna esquerda: identidade e preço -->
                    <div class="flex min-w-0 flex-col gap-4 pb-4">
                        <Field>
                            <FieldLabel>SKU</FieldLabel>
                            <Input v-model="vm.form.sku" />
                        </Field>
                        <Field>
                            <FieldLabel>Descrição</FieldLabel>
                            <Input v-model="vm.form.descricao" />
                        </Field>
                        <div class="grid grid-cols-2 gap-3">
                            <Field>
                                <FieldLabel>NCM</FieldLabel>
                                <Input v-model="vm.form.ncm" />
                            </Field>
                            <Field>
                                <FieldLabel>Unidade</FieldLabel>
                                <Input v-model="vm.form.unidade" />
                            </Field>
                        </div>
                        <div class="grid grid-cols-2 gap-3">
                            <Field>
                                <FieldLabel>Marca</FieldLabel>
                                <Input v-model="vm.form.marca" placeholder="Bosch, NGK..." />
                            </Field>
                            <Field>
                                <FieldLabel>Categoria</FieldLabel>
                                <Input v-model="vm.form.categoria" />
                            </Field>
                        </div>
                        <div class="grid grid-cols-2 gap-3">
                            <Field>
                                <FieldLabel>Preço de custo</FieldLabel>
                                <InputMoney v-model="vm.form.preco_custo" />
                            </Field>
                            <Field>
                                <FieldLabel>Preço de venda</FieldLabel>
                                <InputMoney v-model="vm.form.preco_venda" />
                            </Field>
                        </div>
                        <Field>
                            <FieldLabel>Classe tributária (cClassTrib)</FieldLabel>
                            <Select v-model="vm.form.classe_trib">
                                <SelectTrigger class="w-full">
                                    <SelectValue placeholder="Tributação integral (padrão)" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem v-for="c in vm.classesTributarias" :key="c.c_class_trib" :value="c.c_class_trib">
                                        {{ c.c_class_trib }} — {{ c.descricao }}
                                    </SelectItem>
                                </SelectContent>
                            </Select>
                            <p class="text-xs text-muted-foreground">
                                Enquadramento de IBS/CBS da reforma tributária. Vazio = tributação integral.
                            </p>
                        </Field>
                        <div class="flex items-center gap-2">
                            <Checkbox id="controla-estoque" v-model="vm.form.controla_estoque" />
                            <label for="controla-estoque" class="text-sm cursor-pointer">
                                Controla estoque
                                <span class="text-muted-foreground">— desmarque para serviços/mão de obra</span>
                            </label>
                        </div>
                    </div>

                    <!-- Coluna direita: apoio à decisão de preço -->
                    <div class="flex min-w-0 flex-col gap-4">
                        <PainelPrecificacao
                            :custo-centavos="Math.round((vm.form.preco_custo || 0) * 100)"
                            :categoria="vm.form.categoria || null"
                            :produto-id="vm.editando?.produto_id ?? null"
                            :preco-digitado-centavos="Math.round((vm.form.preco_venda || 0) * 100)"
                            :preco-vigente-centavos="vm.editando?.preco_venda ?? null"
                            :sku="vm.form.sku"
                            :descricao="vm.form.descricao"
                            permitir-registro-concorrencia
                            @usar-sugestao="(centavos) => (vm.form.preco_venda = centavos / 100)"
                        />

                        <!-- Ajustes de precificação deste produto (overrides opcionais) -->
                        <div v-if="vm.editando" class="rounded-lg border p-4">
                            <button
                                type="button"
                                class="flex w-full items-center gap-2 text-left font-medium"
                                @click="ajustesAbertos = !ajustesAbertos"
                            >
                                <ChevronDown v-if="ajustesAbertos" class="size-4" />
                                <ChevronRight v-else class="size-4" />
                                Ajustes deste produto (opcionais)
                            </button>
                            <template v-if="ajustesAbertos">
                                <p class="mb-3 mt-2 text-sm text-muted-foreground">
                                    Campos vazios usam o valor da categoria ou o padrão da loja.
                                </p>
                                <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
                                    <Field>
                                        <FieldLabel class="text-xs text-muted-foreground">Quer ganhar (%)</FieldLabel>
                                        <InputPercent v-model="vm.ajustesProduto.margemPct" :min="0" :max="99" :max-fraction-digits="2" />
                                    </Field>
                                    <Field>
                                        <FieldLabel class="text-xs text-muted-foreground">Custo fixo/unidade</FieldLabel>
                                        <InputMoney v-model="vm.ajustesProduto.custoFixoUnitario" />
                                    </Field>
                                    <Field>
                                        <FieldLabel class="text-xs text-muted-foreground">Frete de venda (%)</FieldLabel>
                                        <InputPercent v-model="vm.ajustesProduto.freteVendaPct" :min="0" :max="99" :max-fraction-digits="2" />
                                    </Field>
                                </div>
                            </template>
                        </div>
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="ghost" @click="vm.dialogVisible = false">Cancelar</Button>
                    <Button :disabled="vm.salvando" @click="vm.salvar">
                        <LoaderCircle v-if="vm.salvando" class="size-4 animate-spin" />
                        <Check v-else class="size-4" />
                        Salvar
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    </div>
</template>
