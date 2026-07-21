<template>
    <div class="pdv-shell" role="application" aria-label="Terminal PDV Finledger">
        <!-- ── Cabeçalho ── -->
        <header class="pdv-header" role="banner">
            <div class="flex items-center gap-3">
                <div class="pdv-logo-wrapper">
                    <ShoppingBag class="pdv-logo-icon" aria-hidden="true" />
                    <span class="pdv-logo">PDV</span>
                </div>
                <StatusBadge
                    v-if="documentoId"
                    :value="`#${documentoId.slice(0, 8)}`"
                    severity="info"
                    class="pdv-sale-tag"
                />
                <StatusBadge
                    v-else
                    :value="modo === 'venda' ? 'Sem venda ativa' : 'Sem orçamento ativo'"
                    severity="warn"
                    class="pdv-sale-tag"
                />
            </div>
            <div
                class="flex items-center gap-2"
                role="toolbar"
                aria-label="Ações principais"
            >
                <div class="pdv-doc-group" role="group" aria-label="Novo documento">
                    <Button
                        size="sm"
                        :variant="modo === 'venda' ? 'default' : 'outline'"
                        :disabled="!podeMudarModo"
                        aria-keyshortcuts="F2"
                        aria-label="Nova Venda (F2)"
                        @click="novaVendaDireta"
                    >
                        <ShoppingCart class="size-4" />
                        Nova Venda
                    </Button>
                    <Button
                        size="sm"
                        :variant="modo === 'orcamento' ? 'default' : 'outline'"
                        :disabled="!podeMudarModo"
                        aria-keyshortcuts="F3"
                        aria-label="Novo Orçamento (F3)"
                        @click="novoOrcamentoDireto"
                    >
                        <FileEdit class="size-4" />
                        Novo Orçamento
                    </Button>
                </div>
                <Button
                    size="sm"
                    variant="outline"
                    :disabled="!podeMudarModo"
                    aria-keyshortcuts="F6"
                    aria-label="Recuperar venda ou orçamento em aberto (F6)"
                    @click="abrirRecuperar"
                >
                    <History class="size-4" />
                    Recuperar
                </Button>
                <Button
                    size="sm"
                    variant="outline"
                    aria-keyshortcuts="F4"
                    aria-label="Consultar estoque e preço de um produto (F4)"
                    @click="abrirConsulta"
                >
                    <Search class="size-4" />
                    Consultar Estoque
                </Button>
                <Button
                    size="icon"
                    variant="ghost"
                    :aria-label="
                        darkMode
                            ? 'Mudar para tema claro'
                            : 'Mudar para tema escuro'
                    "
                    :aria-pressed="darkMode"
                    :title="darkMode ? 'Tema claro' : 'Tema escuro'"
                    @click="toggleDark"
                >
                    <Sun v-if="darkMode" class="size-4" />
                    <Moon v-else class="size-4" />
                </Button>
            </div>
        </header>

        <!-- ── Corpo: dois painéis ── -->
        <div class="pdv-body" role="main">
            <!-- Painel esquerdo: busca + itens -->
            <section class="pdv-left" aria-label="Itens da venda">
                <!-- Barra de busca -->
                <div
                    class="pdv-search-bar"
                    role="search"
                    aria-label="Adicionar produto"
                >
                    <div class="pdv-search-input-wrapper">
                        <AutocompleteRoot
                            v-model="buscaTexto"
                            ignore-filter
                            :open="sugestoesAbertas"
                            class="pdv-autocomplete-root"
                            @update:open="sugestoesAbertas = $event"
                        >
                            <AutocompleteAnchor as-child>
                                <div ref="searchRef" class="relative">
                                    <Search class="pdv-search-icon" aria-hidden="true" />
                                    <AutocompleteInput
                                        placeholder="Buscar produto (SKU ou descrição)…"
                                        class="pdv-autocomplete-input"
                                        :disabled="!documentoId || !documentoEmAberto"
                                        aria-label="Buscar produto por SKU ou descrição"
                                        aria-autocomplete="list"
                                        @update:model-value="onBusca"
                                        @keydown.enter.prevent="adicionarDaBusca"
                                    />
                                </div>
                            </AutocompleteAnchor>
                            <AutocompleteContent class="pdv-autocomplete-content">
                                <AutocompleteViewport>
                                    <AutocompleteEmpty class="pdv-autocomplete-empty">
                                        Nenhum produto encontrado.
                                    </AutocompleteEmpty>
                                    <AutocompleteItem
                                        v-for="s in sugestoes"
                                        :key="s.produto.produto_id"
                                        :value="s.label"
                                        class="pdv-autocomplete-item"
                                        @select="onSelecionarProduto(s)"
                                    >
                                        {{ s.label }}
                                    </AutocompleteItem>
                                </AutocompleteViewport>
                            </AutocompleteContent>
                        </AutocompleteRoot>
                    </div>
                    <InputQuantity
                        ref="qtdRef"
                        v-model="qtd"
                        :min="1"
                        :max="999"
                        class="pdv-qty"
                        :disabled="!documentoId || !documentoEmAberto"
                        aria-label="Quantidade"
                        @keydown.enter.prevent="adicionarItem"
                    />
                    <Button
                        v-if="estoqueInsuficiente"
                        size="sm"
                        :variant="venderSemEstoque ? 'default' : 'outline'"
                        @click="venderSemEstoque = !venderSemEstoque"
                    >
                        <AlertTriangle class="size-4" />
                        {{ venderSemEstoque ? 'Sob encomenda ✓' : 'Sem estoque — confirmar?' }}
                    </Button>
                </div>

                <!-- Lista de itens -->
                <div
                    class="pdv-items-wrapper"
                    role="region"
                    aria-label="Lista de itens"
                >
                    <table class="pdv-items-table" aria-label="Itens adicionados">
                        <thead>
                            <tr>
                                <th class="pdv-th-idx">#</th>
                                <th>SKU</th>
                                <th>Descrição</th>
                                <th class="text-right">Qtd</th>
                                <th class="text-right">Unit.</th>
                                <th class="text-right">Subtotal</th>
                                <th class="pdv-th-remove"><span class="sr-only">Remover</span></th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-if="!itensAtivos.length">
                                <td colspan="7">
                                    <div class="pdv-empty" role="status" aria-live="polite">
                                        <div class="pdv-empty-icon-wrapper">
                                            <ShoppingCart class="pdv-empty-icon" aria-hidden="true" />
                                        </div>
                                        <p class="pdv-empty-title">Nenhum item adicionado</p>
                                        <p class="pdv-empty-hint">
                                            Digite no campo acima para buscar produtos
                                        </p>
                                    </div>
                                </td>
                            </tr>
                            <tr
                                v-for="(item, index) in itensAtivos"
                                v-else
                                :key="item.item_id"
                                :class="itemRowClass(item)"
                            >
                                <td class="pdv-th-idx">
                                    <span class="pdv-idx" aria-hidden="true">{{ index + 1 }}</span>
                                </td>
                                <td><span class="pdv-sku">{{ item.sku }}</span></td>
                                <td><span class="pdv-desc">{{ item.descricao }}</span></td>
                                <td class="text-right">
                                    <span class="font-semibold pdv-qty-cell">{{ item.quantidade }}</span>
                                </td>
                                <td class="text-right">
                                    <span class="pdv-price-cell">{{ formatCentavos(item.preco_unitario_centavos) }}</span>
                                </td>
                                <td class="text-right">
                                    <span class="font-bold pdv-subtotal-cell">{{
                                        formatCentavos(item.preco_unitario_centavos * item.quantidade)
                                    }}</span>
                                </td>
                                <td class="pdv-th-remove">
                                    <Button
                                        variant="ghost"
                                        size="icon-sm"
                                        class="pdv-remove-btn text-destructive"
                                        :disabled="!emAndamento"
                                        :aria-label="`Remover ${item.descricao}`"
                                        @click="removerItem(item)"
                                    >
                                        <Trash2 class="size-4" />
                                    </Button>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <!-- Barra de atalhos -->
                <div
                    class="pdv-shortcuts"
                    role="complementary"
                    aria-label="Atalhos de teclado"
                >
                    <span
                        v-for="a in atalhos"
                        :key="a.key"
                        class="pdv-shortcut"
                    >
                        <kbd>{{ a.key }}</kbd>
                        <span>{{ a.desc }}</span>
                    </span>
                </div>
            </section>

            <!-- Painel direito: cliente + total + pagamento -->
            <aside class="pdv-right" aria-label="Resumo e pagamento">
                <!-- Cliente -->
                <div class="pdv-section pdv-customer-section">
                    <div id="cliente-label" class="pdv-section-label">
                        <User class="size-4" aria-hidden="true" /> Cliente
                    </div>
                    <AutocompleteRoot
                        v-model="clienteInput"
                        ignore-filter
                        :open="sugestoesClienteAbertas"
                        class="w-full"
                        @update:open="sugestoesClienteAbertas = $event"
                    >
                        <AutocompleteAnchor as-child>
                            <div class="relative">
                                <AutocompleteInput
                                    placeholder="Consumidor final"
                                    class="pdv-cliente-input"
                                    :disabled="!documentoId || !documentoEmAberto"
                                    aria-labelledby="cliente-label"
                                    aria-label="Selecionar cliente"
                                    @update:model-value="onBuscaCliente"
                                />
                                <Button
                                    v-if="clienteInput"
                                    type="button"
                                    variant="ghost"
                                    size="icon-sm"
                                    class="pdv-cliente-clear"
                                    aria-label="Limpar cliente"
                                    @click.stop="onLimparCliente"
                                >
                                    <X class="size-3.5" />
                                </Button>
                            </div>
                        </AutocompleteAnchor>
                        <AutocompleteContent class="pdv-autocomplete-content">
                            <AutocompleteViewport>
                                <AutocompleteEmpty class="pdv-autocomplete-empty">
                                    Nenhum cliente encontrado.
                                </AutocompleteEmpty>
                                <AutocompleteItem
                                    v-for="s in sugestoesCliente"
                                    :key="s.cliente.cliente_id"
                                    :value="s.label"
                                    class="pdv-autocomplete-item"
                                    @select="onSelecionarCliente(s)"
                                >
                                    {{ s.label }}
                                </AutocompleteItem>
                            </AutocompleteViewport>
                        </AutocompleteContent>
                    </AutocompleteRoot>
                </div>

                <!-- Total -->
                <div
                    class="pdv-total-section"
                    role="region"
                    aria-label="Total do documento"
                >
                    <div class="pdv-total-label">{{ modo === "venda" ? "Total da Venda" : "Total do Orçamento" }}</div>
                    <div
                        class="pdv-total-value"
                        aria-live="polite"
                        aria-atomic="true"
                    >
                        {{ formatCentavos(total) }}
                    </div>
                    <div v-if="itensAtivos.length" class="pdv-total-count">
                        <Tag class="size-4" aria-hidden="true" />
                        {{ itensAtivos.length }} item{{
                            itensAtivos.length !== 1 ? "s" : ""
                        }}
                    </div>
                </div>

                <!-- Validade (só orçamento) -->
                <div v-if="modo === 'orcamento'" class="pdv-section">
                    <label class="pdv-section-label" for="validade-input">
                        <Calendar class="size-4" aria-hidden="true" /> Validade (dias)
                    </label>
                    <Input
                        id="validade-input"
                        v-model.number="validadeDias"
                        type="number"
                        min="1"
                        :disabled="!!documentoId"
                        aria-label="Validade do orçamento em dias"
                        class="w-full"
                    />
                </div>

                <!-- Pagamento (só venda) -->
                <div v-if="modo === 'venda'" class="pdv-section pdv-payment-section">
                    <div id="pgto-label" class="pdv-section-label">
                        <CreditCard class="size-4" aria-hidden="true" /> Forma
                        de pagamento
                    </div>
                    <div class="pdv-formas" role="group" aria-labelledby="pgto-label">
                        <button
                            v-for="(option, index) in formas"
                            :key="option.value"
                            type="button"
                            class="pdv-forma-btn"
                            :class="{ 'pdv-forma-btn-checked': formaSelecionada === option.value }"
                            :disabled="!documentoId || !documentoEmAberto"
                            @click="selecionarForma(option.value)"
                        >
                            <span class="pdv-forma-key" aria-hidden="true">{{ index + 1 }}</span>
                            <component :is="formaIcon(option.value)" class="size-4" aria-hidden="true" />
                            <span>{{ option.label }}</span>
                        </button>
                    </div>

                    <div
                        v-if="formaSelecionada === 'CartaoCredito'"
                        class="pdv-extra-input"
                    >
                        <label class="pdv-section-label" for="parcelas-input">Parcelas</label>
                        <Input
                            id="parcelas-input"
                            v-model.number="parcelas"
                            type="number"
                            min="1"
                            max="12"
                            aria-label="Número de parcelas"
                            @keydown.enter.prevent="definirPagamento"
                        />
                    </div>
                    <div
                        v-if="formaSelecionada === 'Prazo'"
                        class="pdv-extra-input"
                    >
                        <label class="pdv-section-label" for="prazo-input">Dias para pagamento</label>
                        <Input
                            id="prazo-input"
                            v-model.number="prazoDias"
                            type="number"
                            min="1"
                            aria-label="Prazo em dias"
                            @keydown.enter.prevent="definirPagamento"
                        />
                    </div>

                    <StatusBadge
                        v-if="venda?.forma_pagamento"
                        :value="`✓ ${venda.forma_pagamento}`"
                        severity="success"
                        class="w-full text-center mt-2 pdv-payment-tag"
                        role="status"
                        :aria-label="`Pagamento definido: ${venda.forma_pagamento}`"
                    />
                </div>

                <!-- Confirmar / Cancelar -->
                <div class="pdv-section pdv-actions-section">
                    <Button
                        size="lg"
                        class="w-full pdv-final-confirm-btn"
                        :disabled="!podeConfirmar"
                        aria-keyshortcuts="F10"
                        :aria-label="(modo === 'venda' ? 'Confirmar e finalizar venda' : 'Emitir orçamento') + ' (F10)'"
                        @click="confirmarDocumento"
                    >
                        <CheckCircle2 class="size-5" />
                        {{ (modo === 'venda' ? 'Confirmar Venda' : 'Emitir Orçamento') + ' (F10)' }}
                    </Button>
                    <Button
                        v-if="documentoId && documentoEmAberto"
                        variant="ghost"
                        class="w-full mt-2 text-destructive"
                        :aria-label="modo === 'venda' ? 'Cancelar venda atual' : 'Cancelar orçamento atual'"
                        @click="cancelarVisible = true"
                    >
                        <X class="size-4" />
                        {{ modo === 'venda' ? 'Cancelar venda' : 'Cancelar orçamento' }}
                    </Button>
                </div>
            </aside>
        </div>

        <!-- ── Modal recuperar documento em aberto ── -->
        <PdvRecuperarDialog
            v-model:visible="recuperarVisible"
            :carregando="carregandoRecuperaveis"
            :vendas="vendasRecuperaveis"
            :orcamentos="orcamentosRecuperaveis"
            :cliente-label="clienteLabelRecuperar"
            :format-centavos="formatCentavos"
            @retomar-venda="retomarVenda"
            @retomar-orcamento="retomarOrcamento"
        />

        <!-- ── Modal cancelar ── -->
        <PdvCancelarDialog v-model:visible="cancelarVisible" @cancelar="cancelar" />

        <!-- ── Modal venda/orçamento finalizado ── -->
        <PdvSucessoDialog
            v-model:visible="sucessoVisible"
            :total-finalizado="totalFinalizado"
            :titulo="modo === 'venda' ? 'Venda Finalizada!' : 'Orçamento Emitido!'"
            :mensagem="modo === 'venda' ? 'Venda registrada com sucesso.' : 'Orçamento emitido com sucesso.'"
            :rotulo-novo="(modo === 'venda' ? 'Nova Venda' : 'Novo Orçamento') + ' (F1)'"
            :rotulo-imprimir="modo === 'venda' ? 'Imprimir Cupom' : 'Imprimir Orçamento'"
            @nova-venda="novoDocumentoAposConfirmar"
            @imprimir="imprimirCupom"
        />

        <!-- ── Modal consulta de estoque/preço ── -->
        <Dialog v-model:open="consultaVisible">
            <DialogContent class="sm:max-w-3xl">
                <DialogHeader>
                    <DialogTitle>Consultar Estoque</DialogTitle>
                </DialogHeader>
                <InputGroup class="mb-3 w-full">
                    <InputGroupAddon>
                        <Search class="size-4 text-muted-foreground" />
                    </InputGroupAddon>
                    <InputGroupInput
                        v-model="consultaTermo"
                        placeholder="Buscar por SKU, descrição ou marca…"
                        autofocus
                        aria-label="Buscar produto para consultar estoque e preço"
                    />
                </InputGroup>
                <div class="max-h-96 overflow-y-auto">
                    <AppDataTable
                        :rows="resultadosConsulta"
                        row-key="produto_id"
                        :empty-text="consultaTermo.trim() ? 'Nenhum produto encontrado.' : 'Digite para buscar.'"
                        :page-size-options="[]"
                        :columns="[
                            { key: 'sku', label: 'SKU' },
                            { key: 'descricao', label: 'Descrição' },
                            { key: 'estoque', label: 'Estoque' },
                            { key: 'preco', label: 'Preço', align: 'right' },
                        ]"
                    >
                        <template #cell-estoque="{ row }">
                            {{ row.controla_estoque ? (stockByProduct.get(row.produto_id) ?? 0) : "serviço" }}
                        </template>
                        <template #cell-preco="{ row }">{{ formatCentavos(row.preco_venda) }}</template>
                    </AppDataTable>
                </div>
            </DialogContent>
        </Dialog>

        <Toaster />
    </div>
</template>

<script setup lang="ts">
import {
    AlertTriangle,
    Banknote,
    Calendar,
    CheckCircle2,
    Clock,
    CreditCard,
    FileEdit,
    History,
    Moon,
    QrCode,
    Search,
    ShoppingBag,
    ShoppingCart,
    Sun,
    Tag,
    Trash2,
    User,
    X,
} from '@lucide/vue'
import {
    AutocompleteAnchor,
    AutocompleteContent,
    AutocompleteEmpty,
    AutocompleteInput,
    AutocompleteItem,
    AutocompleteRoot,
    AutocompleteViewport,
} from 'reka-ui'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { Toaster } from '@/components/ui/sonner'

definePageMeta({ layout: false })

// `toRefs` preserva a reatividade dos campos desestruturados — desestruturar
// direto de `vm` (reactive()) copiaria valores primitivos "congelados" no
// momento do setup, e o template pararia de atualizar após qualquer
// reatribuição de referência (ex.: `venda.value = novaVenda` dentro do
// viewmodel não refletiria aqui).
const vm = useTerminalViewModel()
const {
    darkMode,
    toggleDark,
    iniciar,
    encerrar,
    produtos,
    clientes,
    modo,
    validadeDias,
    podeMudarModo,
    venda,
    itens,
    orcamento,
    itensAtivos,
    documentoId,
    emAndamento,
    documentoEmAberto,
    total,
    podeConfirmar,
    itemRowClass,
    novoDocumento,
    novaVendaDireta,
    novoOrcamentoDireto,
    recuperarVisible,
    carregandoRecuperaveis,
    vendasRecuperaveis,
    orcamentosRecuperaveis,
    abrirRecuperar,
    retomarVenda,
    retomarOrcamento,
    searchRef,
    qtdRef,
    busca,
    sugestoes,
    qtd,
    venderSemEstoque,
    estoqueInsuficiente,
    onBusca: onBuscaVm,
    onSelecionarProduto: onSelecionarProdutoVm,
    adicionarDaBusca,
    adicionarItem,
    removerItem,
    clienteInput,
    sugestoesCliente,
    onBuscaCliente: onBuscaClienteVm,
    onSelecionarCliente: onSelecionarClienteVm,
    onLimparCliente,
    formaSelecionada,
    parcelas,
    prazoDias,
    formas,
    selecionarForma,
    definirPagamento,
    sucessoVisible,
    totalFinalizado,
    confirmarDocumento,
    novoDocumentoAposConfirmar,
    imprimirCupom,
    cancelarVisible,
    cancelar,
    consultaVisible,
    consultaTermo,
    resultadosConsulta,
    abrirConsulta,
    stockByProduct,
    atalhos,
    formatCentavos,
} = toRefs(vm)

// Rótulo do cliente para a lista de recuperação: nome do cadastro, senão o
// avulso do orçamento, senão consumidor final.
function clienteLabelRecuperar(clienteId: string | null, clienteAvulso?: string | null): string {
    if (clienteId) {
        return clientes.value.find((c) => c.cliente_id === clienteId)?.nome ?? 'Cliente'
    }
    return clienteAvulso || 'Consumidor final'
}

// --- Adaptação do padrão PrimeVue AutoComplete (query-driven, texto livre)
// para reka-ui AutocompleteRoot/Item, que já era usado nos componentes
// SearchSelect/Combobox deste projeto. `busca`/`clienteInput` no viewmodel
// guardam o item selecionado (ou texto bruto); `buscaTexto`/`clienteInput` são
// o v-model do AutocompleteRoot, que no reka É o texto da caixa. CUIDADO: o
// AutocompleteRoot usa o `:value` do item selecionado COMO o texto exibido
// (ListboxRoot → contextModelValue → texto). Por isso o `:value` de cada item
// tem de ser o RÓTULO, não o produto_id — senão o UUID vaza para a caixa ao
// selecionar. O `@select` ainda recebe o objeto `s` inteiro, então temos o
// produto/cliente para a lógica; o `:value` serve só ao texto/seleção do reka.
const buscaTexto = ref('')
const sugestoesAbertas = ref(false)
const sugestoesClienteAbertas = ref(false)

watch(busca, (v) => {
    if (v == null) buscaTexto.value = ''
})

// Ao selecionar um item, o reka reescreve o texto da caixa para o rótulo e
// dispara @update:model-value → onBusca(rótulo). Sem isto, o dropdown reabriria
// logo após a escolha. A flag suprime essa reabertura reflexa.
let selecionandoProduto = false
let selecionandoCliente = false

function onBusca(query: string) {
    if (selecionandoProduto) return
    buscaTexto.value = query
    onBuscaVm.value({ query })
    sugestoesAbertas.value = query.trim().length > 0
}

function onSelecionarProduto(s: { label: string, produto: any }) {
    selecionandoProduto = true
    onSelecionarProdutoVm.value({ value: s })
    busca.value = s
    buscaTexto.value = s.label
    sugestoesAbertas.value = false
    nextTick(() => { selecionandoProduto = false })
}

function onBuscaCliente(query: string) {
    if (selecionandoCliente) return
    clienteInput.value = query
    onBuscaClienteVm.value({ query })
    sugestoesClienteAbertas.value = query.trim().length > 0
}

function onSelecionarCliente(s: { label: string, cliente: any }) {
    selecionandoCliente = true
    onSelecionarClienteVm.value({ value: s })
    clienteInput.value = s.label
    sugestoesClienteAbertas.value = false
    nextTick(() => { selecionandoCliente = false })
}

const formaIcons: Record<string, any> = {
    Dinheiro: Banknote,
    CartaoDebito: CreditCard,
    CartaoCredito: CreditCard,
    Pix: QrCode,
    Prazo: Clock,
}

function formaIcon(value: string) {
    return formaIcons[value] ?? CreditCard
}

onMounted(() => {
    iniciar.value()
})

onUnmounted(() => {
    encerrar.value()
})
</script>

<style scoped>
/* ── Estrutura geral ── */
.pdv-shell {
    display: flex;
    flex-direction: column;
    height: 100dvh;
    background: var(--background);
    color: var(--foreground);
    overflow: hidden;
    font-size: 1rem;
}

/* ── Cabeçalho ── */
.pdv-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.5rem;
    background: var(--card);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 1rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
}

.pdv-logo-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.pdv-logo-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: var(--primary);
}

.pdv-logo {
    font-size: 1.25rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    color: var(--primary);
}

.pdv-sale-tag {
    font-weight: 600;
}

.pdv-doc-group {
    display: flex;
    gap: 0.375rem;
    padding: 0.25rem;
    background: var(--background);
    border-radius: var(--radius-lg, 8px);
}

/* ── Corpo ── */
.pdv-body {
    display: flex;
    flex: 1;
    overflow: hidden;
}

/* ── Painel esquerdo ── */
.pdv-left {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    border-right: 1px solid var(--border);
}

/* ── Barra de busca ── */
.pdv-search-bar {
    display: flex;
    align-items: stretch;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    background: var(--card);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
}

.pdv-search-input-wrapper {
    position: relative;
    flex: 1 1 auto;
    min-width: 0;
}

.pdv-autocomplete-root {
    width: 100%;
}

.pdv-search-icon {
    position: absolute;
    left: 0.875rem;
    top: 50%;
    transform: translateY(-50%);
    width: 1rem;
    height: 1rem;
    color: var(--muted-foreground);
    z-index: 1;
    pointer-events: none;
}

.pdv-autocomplete-input {
    width: 100%;
    height: 3rem;
    font-size: 1rem;
    padding-left: 2.5rem;
    padding-right: 0.875rem;
    border-radius: var(--radius-lg, 8px);
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--foreground);
    outline: none;
}

.pdv-autocomplete-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.pdv-autocomplete-input:focus-visible {
    border-color: var(--ring);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--ring) 30%, transparent);
}

.pdv-autocomplete-content {
    z-index: 50;
    width: var(--reka-autocomplete-trigger-width, 100%);
    max-height: 24rem;
    overflow: auto;
    background: var(--popover);
    color: var(--popover-foreground);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 8px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    padding: 0.25rem;
}

.pdv-autocomplete-item {
    display: flex;
    align-items: center;
    padding: 0.5rem 0.625rem;
    font-size: 0.875rem;
    border-radius: calc(var(--radius-lg, 8px) - 2px);
    cursor: pointer;
    outline: none;
}

.pdv-autocomplete-item[data-highlighted] {
    background: var(--muted);
}

.pdv-autocomplete-empty {
    padding: 0.75rem;
    text-align: center;
    font-size: 0.875rem;
    color: var(--muted-foreground);
}

.pdv-cliente-input {
    width: 100%;
    height: 2.25rem;
    font-size: 0.875rem;
    padding: 0 2rem 0 0.75rem;
    border-radius: var(--radius-lg, 8px);
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--foreground);
    outline: none;
}

.pdv-cliente-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.pdv-cliente-input:focus-visible {
    border-color: var(--ring);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--ring) 30%, transparent);
}

.pdv-cliente-clear {
    position: absolute;
    right: 0.25rem;
    top: 50%;
    transform: translateY(-50%);
}

.pdv-qty {
    flex: 0 0 8rem;
}

.pdv-qty :deep(input) {
    height: 3rem;
    font-size: 1rem;
    font-weight: 600;
}

/* ── Lista de itens ── */
.pdv-items-wrapper {
    flex: 1;
    overflow-y: auto;
    padding: 0;
}

.pdv-items-table {
    width: 100%;
    font-size: 0.95rem;
    border-collapse: collapse;
}

.pdv-items-table thead th {
    position: sticky;
    top: 0;
    background: var(--muted);
    z-index: 1;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
    padding: 0.75rem 1rem;
    text-align: left;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted-foreground);
    border-bottom: 2px solid var(--border);
    white-space: nowrap;
}

.pdv-th-idx {
    width: 3rem;
}

.pdv-th-remove {
    width: 3rem;
}

.pdv-items-table tbody td {
    padding: 0.875rem 1rem;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
}

.pdv-items-table tbody tr {
    transition: background 0.15s;
}

.pdv-items-table tbody tr:hover {
    background: var(--muted);
}

.pdv-items-table tbody tr.pdv-last-item {
    background: color-mix(in srgb, var(--primary) 8%, transparent);
    border-left: 3px solid var(--primary);
}

.pdv-idx {
    color: var(--muted-foreground);
    font-size: 0.85rem;
    font-weight: 600;
}

.pdv-sku {
    font-family: "Courier New", monospace;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--muted-foreground);
}

.pdv-desc {
    font-weight: 500;
}

.pdv-qty-cell {
    font-size: 1rem;
}

.pdv-price-cell {
    font-size: 0.9rem;
}

.pdv-subtotal-cell {
    font-size: 1rem;
    color: var(--primary);
}

.pdv-remove-btn {
    opacity: 0.6;
    transition: opacity 0.15s;
}

.pdv-remove-btn:hover:not(:disabled) {
    opacity: 1;
}

/* ── Empty state ── */
.pdv-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 20rem;
    padding: 3rem;
    gap: 0.75rem;
}

.pdv-empty-icon-wrapper {
    width: 5rem;
    height: 5rem;
    border-radius: 50%;
    background: var(--muted);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 1rem;
}

.pdv-empty-icon {
    width: 2.5rem;
    height: 2.5rem;
    color: var(--muted-foreground);
    opacity: 0.5;
}

.pdv-empty-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--foreground);
}

.pdv-empty-hint {
    font-size: 0.875rem;
    color: var(--muted-foreground);
}

/* ── Barra de atalhos ── */
.pdv-shortcuts {
    display: flex;
    flex-wrap: wrap;
    gap: 1.25rem;
    padding: 0.75rem 1.25rem;
    background: var(--card);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
}

.pdv-shortcut {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem;
    color: var(--muted-foreground);
}

kbd {
    background: var(--muted);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 0.75rem;
    font-family: monospace;
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}

/* ── Painel direito ── */
.pdv-right {
    width: 24rem;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    background: var(--card);
}

.pdv-section {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border);
}

.pdv-section-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted-foreground);
    margin-bottom: 0.75rem;
}

/* ── Total ── */
.pdv-total-section {
    padding: 2rem 1.5rem;
    text-align: center;
    border-bottom: 1px solid var(--border);
}

.pdv-total-label {
    font-size: 0.8rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--muted-foreground);
    margin-bottom: 0.5rem;
}

.pdv-total-value {
    font-size: clamp(2.5rem, 6vw, 4rem);
    font-weight: 800;
    color: var(--primary);
    font-variant-numeric: tabular-nums;
    line-height: 1.1;
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
}

.pdv-total-count {
    font-size: 0.9rem;
    color: var(--muted-foreground);
    margin-top: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
}

/* ── Formas de pagamento ── */

.pdv-formas {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
}

.pdv-forma-btn {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 0.625rem;
    padding: 0.75rem 0.875rem;
    background: var(--background);
    border: 2px solid var(--border);
    border-radius: var(--radius-lg, 8px);
    color: var(--foreground);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    text-align: left;
    width: 100%;
    font-family: inherit;
}

.pdv-forma-btn:not(:disabled):hover {
    background: var(--muted);
    border-color: var(--primary);
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.pdv-forma-btn-checked {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.pdv-forma-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.pdv-forma-key {
    font-size: 0.7rem;
    background: rgba(0, 0, 0, 0.15);
    border-radius: 4px;
    padding: 2px 6px;
    font-family: monospace;
    font-weight: 700;
    min-width: 1.25rem;
    text-align: center;
    flex-shrink: 0;
}

.pdv-forma-btn-checked .pdv-forma-key {
    background: rgba(255, 255, 255, 0.25);
}

.pdv-extra-input {
    margin-top: 1rem;
}

.pdv-payment-tag {
    font-weight: 600;
    padding: 0.5rem;
}

/* ── Ações finais ── */
.pdv-actions-section {
    padding: 1.5rem;
}

.pdv-final-confirm-btn {
    font-size: 1rem;
    font-weight: 400;
    padding: 1rem;
}

/* ── Utilitário acessibilidade ── */
.sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
}
</style>
