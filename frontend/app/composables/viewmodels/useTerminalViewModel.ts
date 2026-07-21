import type { Cliente } from '~/models/crm'
import { listarClientes } from '~/models/crm'
import type { Produto } from '~/models/catalogo'
import { listarProdutos } from '~/models/catalogo'
import type { Saldo } from '~/models/estoque'
import { listarSaldos } from '~/models/estoque'
import type { Venda, VendaItem } from '~/models/vendas'
import {
    adicionarItemVenda,
    buscarVenda,
    cancelarVenda,
    confirmarVenda as confirmarVendaApi,
    definirFormaPagamento,
    iniciarVenda,
    listarVendas,
    removerItemVenda,
} from '~/models/vendas'
import type { Orcamento, OrcamentoDetalhes, OrcamentoItem } from '~/models/orcamentos'
import {
    adicionarItem as adicionarItemOrcamentoApi,
    cancelarOrcamento as cancelarOrcamentoApi,
    criarOrcamento,
    emitirOrcamento,
    buscarOrcamento,
    listarOrcamentos,
    removerItem as removerItemOrcamentoApi,
} from '~/models/orcamentos'

/** ViewModel do Terminal PDV: concentra estado e regras de negócio da tela de venda
 * (busca/seleção de produto, carrinho, cliente, pagamento, confirmação/cancelamento). */
export function useTerminalViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { formatCentavos } = useFormat()
    const { notifySuccess, notifyWarn, notifyInfo, notifyError } = useNotify()

    // --- Tema claro/escuro ---
    // The PDV keeps its own preference (default = light, better for counters)
    // but applies it through the shared layout theme so <html> stays the
    // single source of truth. `persist: false` keeps the PDV choice from
    // overwriting the user's app-wide theme cookie.
    const STORAGE_KEY = 'pdv-dark-mode'

    const { setDarkTheme, restorePersistedTheme } = useLayout()
    const darkMode = ref(false)

    function toggleDark() {
        darkMode.value = !darkMode.value
        setDarkTheme(darkMode.value, { persist: false })
        localStorage.setItem(STORAGE_KEY, darkMode.value ? '1' : '0')
    }

    function iniciarTema() {
        const saved = localStorage.getItem(STORAGE_KEY)
        darkMode.value = saved === '1'
        setDarkTheme(darkMode.value, { persist: false })
    }

    function encerrarTema() {
        // Restaura o tema global salvo ao sair do terminal
        restorePersistedTheme()
    }

    // --- Dados master ---
    const produtos = ref<Produto[]>([])
    const clientes = ref<Cliente[]>([])
    const saldos = ref<Saldo[]>([])

    const stockByProduct = computed(() => {
        const map = new Map<string, number>()
        saldos.value.forEach((s) => map.set(s.produto_id, s.quantidade))
        return map
    })

    async function carregarMaster() {
        try {
            const [{ produtos: p }, { clientes: c }, { saldos: s }] = await Promise.all([
                listarProdutos(apiFetch),
                listarClientes(apiFetch),
                listarSaldos(apiFetch),
            ])
            produtos.value = p.filter((x) => x.ativo)
            clientes.value = c
            saldos.value = s
        } catch (e) {
            notifyError(apiErrorMessage(e), 'Erro ao carregar dados')
        }
    }

    // --- Modo: venda ou orçamento ---
    // O carrinho/tabela de itens é compartilhado entre os dois modos — só o
    // documento por trás (venda vs. orçamento) muda. Trocar de modo só é
    // permitido sem itens no carrinho (ver `podeMudarModo`), pra não perder
    // um carrinho em andamento por engano.
    const modo = ref<'venda' | 'orcamento'>('venda')
    const validadeDias = ref(15)

    // --- Venda ativa ---
    const venda = ref<Venda | null>(null)
    const itens = ref<VendaItem[]>([])

    // --- Orçamento ativo ---
    const orcamento = ref<OrcamentoDetalhes | null>(null)

    const vendaId = computed(() => venda.value?.venda_id ?? '')
    const orcamentoId = computed(() => orcamento.value?.orcamento.orcamento_id ?? '')
    const documentoId = computed(() => (modo.value === 'venda' ? vendaId.value : orcamentoId.value))

    const itensAtivos = computed<(VendaItem | OrcamentoItem)[]>(() =>
        modo.value === 'venda' ? itens.value : (orcamento.value?.itens ?? []),
    )

    const emAndamento = computed(() => venda.value?.status === 'Em Andamento')
    const orcamentoEmAberto = computed(() => orcamento.value?.orcamento.status === 'Rascunho')
    /** Documento (venda ou orçamento) atual aceita alterações (adicionar/remover item). */
    const documentoEmAberto = computed(() => (modo.value === 'venda' ? emAndamento.value : orcamentoEmAberto.value))

    const total = computed(() => itensAtivos.value.reduce((s, i) => s + i.preco_unitario_centavos * i.quantidade, 0))
    const podeConfirmar = computed(() =>
        modo.value === 'venda'
            ? emAndamento.value && itens.value.length > 0 && !!venda.value?.forma_pagamento
            : orcamentoEmAberto.value && itensAtivos.value.length > 0,
    )
    const podeMudarModo = computed(() => itensAtivos.value.length === 0)

    function itemRowClass(data: VendaItem | OrcamentoItem) {
        const lista = itensAtivos.value
        const ultimo = lista[lista.length - 1]
        return { 'pdv-last-item': !!ultimo && ultimo.item_id === data.item_id }
    }

    async function novoDocumento() {
        try {
            if (modo.value === 'venda') {
                const { venda_id } = await iniciarVenda(apiFetch, null)
                venda.value = await buscarVenda(apiFetch, venda_id).then((r) => r.venda)
                itens.value = []
                orcamento.value = null
                formaSelecionada.value = null
            } else {
                const { orcamento_id } = await criarOrcamento(apiFetch, {
                    cliente_id: null,
                    cliente_avulso: null,
                    validade_dias: validadeDias.value,
                })
                orcamento.value = await buscarOrcamento(apiFetch, orcamento_id)
                venda.value = null
                itens.value = []
            }
            clienteInput.value = ''
            focusBusca()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    /** Atalhos diretos (F2/F3 e botões do cabeçalho): trocam o modo e já
     * iniciam o documento num único passo, em vez de exigir alternar o modo
     * e só depois acionar "novo documento". Sem efeito com um carrinho em
     * andamento (mesma guarda de `podeMudarModo`). */
    async function novaVendaDireta() {
        if (!podeMudarModo.value) return
        modo.value = 'venda'
        await novoDocumento()
    }

    async function novoOrcamentoDireto() {
        if (!podeMudarModo.value) return
        modo.value = 'orcamento'
        await novoDocumento()
    }

    async function recarregar() {
        if (!documentoId.value) return
        try {
            if (modo.value === 'venda') {
                const r = await buscarVenda(apiFetch, vendaId.value)
                venda.value = r.venda
                itens.value = r.itens
            } else {
                orcamento.value = await buscarOrcamento(apiFetch, orcamentoId.value)
            }
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Recuperação de documentos em aberto ---
    // O PDV fica aberto o dia todo; o operador retoma vendas EmAndamento (inclui
    // as geradas ao aceitar um orçamento) e orçamentos Rascunho/Emitido sem sair
    // do terminal.
    const recuperarVisible = ref(false)
    const carregandoRecuperaveis = ref(false)
    const vendasRecuperaveis = ref<Venda[]>([])
    const orcamentosRecuperaveis = ref<Orcamento[]>([])

    async function abrirRecuperar() {
        recuperarVisible.value = true
        carregandoRecuperaveis.value = true
        try {
            const [rv, ro] = await Promise.all([
                listarVendas(apiFetch, { apenasAbertas: true }),
                listarOrcamentos(apiFetch, { apenasAbertos: true }),
            ])
            vendasRecuperaveis.value = rv.vendas
            orcamentosRecuperaveis.value = ro.orcamentos
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            carregandoRecuperaveis.value = false
        }
    }

    /** Retoma uma venda EmAndamento no terminal. Não reconstrói a forma de
     * pagamento a partir da string de exibição (frágil): o operador a repõe
     * antes de confirmar. Guarda igual à troca de modo — não descarta um
     * carrinho com itens. */
    async function retomarVenda(id: string) {
        if (!podeMudarModo.value) {
            notifyWarn('Documento em andamento', 'Finalize ou limpe o atual antes de recuperar outro.')
            return
        }
        try {
            const r = await buscarVenda(apiFetch, id)
            modo.value = 'venda'
            venda.value = r.venda
            itens.value = r.itens
            orcamento.value = null
            formaSelecionada.value = null
            clienteInput.value = ''
            recuperarVisible.value = false
            focusBusca()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    /** Retoma um orçamento Rascunho/Emitido no terminal. */
    async function retomarOrcamento(id: string) {
        if (!podeMudarModo.value) {
            notifyWarn('Documento em andamento', 'Finalize ou limpe o atual antes de recuperar outro.')
            return
        }
        try {
            const o = await buscarOrcamento(apiFetch, id)
            modo.value = 'orcamento'
            orcamento.value = o
            venda.value = null
            itens.value = []
            clienteInput.value = ''
            recuperarVisible.value = false
            focusBusca()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Busca de produto ---
    const searchRef = ref<any>(null)
    const qtdRef = ref<any>(null)
    const busca = ref<any>(null)
    const sugestoes = ref<{ label: string; produto: Produto }[]>([])
    const produtoSelecionado = ref<Produto | null>(null)
    const qtd = ref(1)

    const { buscarAproximado } = useFuzzySearch()

    function onBusca(event: { query: string }) {
        sugestoes.value = buscarAproximado(
            produtos.value,
            event.query,
            (p) => `${p.sku} ${p.descricao} ${p.marca ?? ''}`,
        )
            .slice(0, 12)
            .map((p) => {
                const marca = p.marca ? ` · ${p.marca}` : ''
                const estoqueLabel = p.controla_estoque
                    ? `${stockByProduct.value.get(p.produto_id) ?? 0} em estoque`
                    : 'serviço'
                return {
                    label: `${p.sku} — ${p.descricao}${marca} (${formatCentavos(p.preco_venda)}) · ${estoqueLabel}`,
                    produto: p,
                }
            })
    }

    function onSelecionarProduto(event: { value: { label: string; produto: Produto } }) {
        produtoSelecionado.value = event.value.produto
        venderSemEstoque.value = false
    }

    function adicionarDaBusca() {
        if (sugestoes.value.length === 1) {
            produtoSelecionado.value = sugestoes.value[0]?.produto ?? null
        }
        if (produtoSelecionado.value) adicionarItem()
    }

    /** Produto selecionado controla estoque e a quantidade pedida excede o
     * saldo — o operador precisa confirmar a venda sob encomenda. Só se
     * aplica em modo venda: orçamento nunca bloqueia por estoque no PDV (o
     * backend decide, via a flag "permitir orçamento sem estoque" do tenant). */
    const venderSemEstoque = ref(false)
    const estoqueInsuficiente = computed(() => {
        if (modo.value !== 'venda') return false
        const p = produtoSelecionado.value
        if (!p || !p.controla_estoque) return false
        return (stockByProduct.value.get(p.produto_id) ?? 0) < qtd.value
    })

    async function adicionarItem() {
        if (!documentoId.value || !produtoSelecionado.value) return
        if (estoqueInsuficiente.value && !venderSemEstoque.value) {
            notifyWarn('Estoque insuficiente', 'Confirme "sob encomenda" para vender mesmo assim.')
            return
        }
        const p = produtoSelecionado.value
        try {
            if (modo.value === 'venda') {
                await adicionarItemVenda(apiFetch, vendaId.value, {
                    produto_id: p.produto_id,
                    sku: p.sku,
                    descricao: p.descricao,
                    quantidade: qtd.value,
                    preco_unitario_centavos: p.preco_venda,
                    vender_sem_estoque: venderSemEstoque.value,
                })
            } else {
                await adicionarItemOrcamentoApi(apiFetch, orcamentoId.value, {
                    produto_id: p.produto_id,
                    sku: p.sku,
                    descricao: p.descricao,
                    quantidade: qtd.value,
                    preco_unitario_centavos: p.preco_venda,
                })
            }
            busca.value = null
            produtoSelecionado.value = null
            qtd.value = 1
            venderSemEstoque.value = false
            await recarregar()
            focusBusca()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    async function removerItem(item: VendaItem | OrcamentoItem) {
        try {
            if (modo.value === 'venda') {
                await removerItemVenda(apiFetch, vendaId.value, item.item_id)
            } else {
                await removerItemOrcamentoApi(apiFetch, orcamentoId.value, item.item_id)
            }
            await recarregar()
            focusBusca()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Cliente ---
    const clienteInput = ref<any>(null)
    const sugestoesCliente = ref<{ label: string; cliente: Cliente }[]>([])

    function onBuscaCliente(event: { query: string }) {
        sugestoesCliente.value = buscarAproximado(clientes.value, event.query, (c) => c.nome)
            .slice(0, 8)
            .map((c) => ({ label: c.nome, cliente: c }))
    }

    function onSelecionarCliente(event: { value: { label: string; cliente: Cliente } }) {
        notifyInfo('Cliente', event.value.cliente.nome, 2000)
    }

    function onLimparCliente() {
        clienteInput.value = ''
    }

    // --- Pagamento ---
    const formaSelecionada = ref<string | null>(null)
    const parcelas = ref(2)
    const prazoDias = ref(30)

    const formas = [
        { label: 'Dinheiro', value: 'Dinheiro', icon: 'pi pi-money-bill' },
        { label: 'Débito', value: 'CartaoDebito', icon: 'pi pi-credit-card' },
        { label: 'Crédito', value: 'CartaoCredito', icon: 'pi pi-credit-card' },
        { label: 'Pix', value: 'Pix', icon: 'pi pi-qrcode' },
        { label: 'A prazo', value: 'Prazo', icon: 'pi pi-clock' },
    ]

    function selecionarForma(valor: string) {
        formaSelecionada.value = valor
        if (['Dinheiro', 'Pix', 'CartaoDebito'].includes(valor)) {
            definirPagamento()
        }
    }

    function montarForma() {
        switch (formaSelecionada.value) {
            case 'CartaoCredito':
                return { CartaoCredito: { parcelas: parcelas.value } }
            case 'Prazo':
                return { Prazo: { dias: prazoDias.value } }
            default:
                return formaSelecionada.value
        }
    }

    async function definirPagamento() {
        if (!vendaId.value || !formaSelecionada.value) return
        try {
            await definirFormaPagamento(apiFetch, vendaId.value, montarForma())
            await recarregar()
            notifySuccess('Pagamento definido', undefined, 2000)
            focusBusca()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Confirmar ---
    const sucessoVisible = ref(false)
    const totalFinalizado = ref(0)
    const { printReceipt } = useThermalPrint()
    const { tenantSlug, username } = useAuth()
    const { businessInfo, garantirCarregado: garantirEmpresaCarregada } = useEmpresaInfo()
    void garantirEmpresaCarregada()

    /** Snapshot of the finished document (venda ou orçamento) so the receipt
     * can be printed after the cart state is cleared. */
    const finishedSale = ref<{
        tipo: 'venda' | 'orcamento'
        title: string
        reference: string
        items: (VendaItem | OrcamentoItem)[]
        totalCents: number
        paymentLabel: string
    } | null>(null)

    async function confirmarDocumento() {
        if (!podeConfirmar.value) return
        try {
            if (modo.value === 'venda') {
                await confirmarVendaApi(apiFetch, vendaId.value)
                finishedSale.value = {
                    tipo: 'venda',
                    title: 'CUPOM NÃO FISCAL',
                    reference: `Venda ${vendaId.value.slice(0, 8)}`,
                    items: [...itens.value],
                    totalCents: total.value,
                    paymentLabel: venda.value?.forma_pagamento ?? '',
                }
                venda.value = null
                itens.value = []
            } else {
                await emitirOrcamento(apiFetch, orcamentoId.value)
                finishedSale.value = {
                    tipo: 'orcamento',
                    title: 'ORÇAMENTO',
                    reference: `Orçamento ${orcamentoId.value.slice(0, 8)}`,
                    items: [...itensAtivos.value],
                    totalCents: total.value,
                    paymentLabel: '',
                }
                orcamento.value = null
            }
            totalFinalizado.value = total.value
            sucessoVisible.value = true
            void carregarMaster() // refresh stock quantities shown in the search
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    function imprimirCupom() {
        const doc = finishedSale.value
        if (!doc) return
        printReceipt({
            storeName: tenantSlug.value || 'Finledger',
            title: doc.title,
            reference: doc.reference,
            meta: [{ label: 'Operador', value: username.value }],
            businessInfo: businessInfo.value,
            items: doc.items.map((i) => ({
                descricao: i.descricao,
                sku: i.sku,
                quantidade: i.quantidade,
                unitCents: i.preco_unitario_centavos,
            })),
            totalCents: doc.totalCents,
            paymentLabel: doc.paymentLabel ? `Pagamento: ${doc.paymentLabel}` : undefined,
            footerNote: doc.tipo === 'orcamento' ? 'Orçamento sem valor fiscal. Sujeito a disponibilidade de estoque.' : undefined,
        })
    }

    function novoDocumentoAposConfirmar() {
        sucessoVisible.value = false
        novoDocumento()
    }

    // --- Cancelar ---
    const cancelarVisible = ref(false)

    async function cancelar(motivo: string) {
        if (!documentoId.value) return
        try {
            if (modo.value === 'venda') {
                await cancelarVenda(apiFetch, vendaId.value, motivo)
                venda.value = null
                itens.value = []
                notifyWarn('Venda cancelada')
            } else {
                await cancelarOrcamentoApi(apiFetch, orcamentoId.value, motivo)
                orcamento.value = null
                notifyWarn('Orçamento cancelado')
            }
            cancelarVisible.value = false
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Consulta de estoque/preço ---
    // Independente de venda/orçamento — funciona mesmo sem nenhum documento
    // em andamento, sobre os mesmos `produtos`/`stockByProduct` já carregados.
    const consultaVisible = ref(false)
    const consultaTermo = ref('')
    const resultadosConsulta = computed(() => {
        if (!consultaTermo.value.trim()) return []
        return buscarAproximado(
            produtos.value.filter((p) => p.ativo),
            consultaTermo.value,
            (p) => `${p.sku} ${p.descricao} ${p.marca ?? ''}`,
        ).slice(0, 30)
    })

    function abrirConsulta() {
        consultaTermo.value = ''
        consultaVisible.value = true
    }

    // --- Teclado ---
    function focusBusca() {
        nextTick(() => {
            const el = searchRef.value?.$el?.querySelector('input')
            el?.focus()
        })
    }

    function onKey(e: KeyboardEvent) {
        if (cancelarVisible.value || sucessoVisible.value) return

        switch (e.key) {
            case 'F1':
                e.preventDefault()
                novoDocumento()
                break
            case 'F2':
                e.preventDefault()
                novaVendaDireta()
                break
            case 'F3':
                e.preventDefault()
                novoOrcamentoDireto()
                break
            case 'F4':
                e.preventDefault()
                abrirConsulta()
                break
            case 'F6':
                e.preventDefault()
                abrirRecuperar()
                break
            case 'F10':
                e.preventDefault()
                confirmarDocumento()
                break
            case 'Escape':
                e.preventDefault()
                busca.value = null
                produtoSelecionado.value = null
                focusBusca()
                break
            case '1':
            case '2':
            case '3':
            case '4':
            case '5': {
                const tag = (e.target as HTMLElement)?.tagName
                if (tag === 'INPUT' || tag === 'TEXTAREA') break
                e.preventDefault()
                const idx = parseInt(e.key) - 1
                if (formas[idx] && modo.value === 'venda' && emAndamento.value) {
                    selecionarForma(formas[idx].value)
                }
                break
            }
            case 'Delete':
            case 'Backspace': {
                const tag = (e.target as HTMLElement)?.tagName
                if (tag === 'INPUT' || tag === 'TEXTAREA') break
                const ultimoItem = itensAtivos.value[itensAtivos.value.length - 1]
                if (ultimoItem && documentoEmAberto.value) {
                    e.preventDefault()
                    removerItem(ultimoItem)
                }
                break
            }
        }
    }

    const atalhos = [
        { key: 'F2', desc: 'Nova venda' },
        { key: 'F3', desc: 'Novo orçamento' },
        { key: 'F4', desc: 'Consultar estoque' },
        { key: 'F6', desc: 'Recuperar documento' },
        { key: 'Enter', desc: 'Adicionar item' },
        { key: '1–5', desc: 'Selecionar pagamento' },
        { key: 'F10', desc: 'Confirmar' },
        { key: 'Del', desc: 'Remover último' },
        { key: 'Esc', desc: 'Limpar busca' },
    ]

    function iniciar() {
        iniciarTema()
        document.addEventListener('keydown', onKey)
        carregarMaster()
        focusBusca()
    }

    function encerrar() {
        encerrarTema()
        document.removeEventListener('keydown', onKey)
    }

    return reactive({
        // tema
        darkMode,
        toggleDark,
        // ciclo de vida
        iniciar,
        encerrar,
        // dados master
        produtos,
        clientes,
        // modo (venda/orçamento)
        modo,
        validadeDias,
        podeMudarModo,
        // documento ativo (venda ou orçamento)
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
        recarregar,
        // recuperação de documentos em aberto
        recuperarVisible,
        carregandoRecuperaveis,
        vendasRecuperaveis,
        orcamentosRecuperaveis,
        abrirRecuperar,
        retomarVenda,
        retomarOrcamento,
        // busca de produto
        searchRef,
        qtdRef,
        busca,
        sugestoes,
        produtoSelecionado,
        qtd,
        venderSemEstoque,
        estoqueInsuficiente,
        onBusca,
        onSelecionarProduto,
        adicionarDaBusca,
        adicionarItem,
        removerItem,
        // cliente
        clienteInput,
        sugestoesCliente,
        onBuscaCliente,
        onSelecionarCliente,
        onLimparCliente,
        // pagamento
        formaSelecionada,
        parcelas,
        prazoDias,
        formas,
        selecionarForma,
        definirPagamento,
        // confirmar
        sucessoVisible,
        totalFinalizado,
        confirmarDocumento,
        novoDocumentoAposConfirmar,
        imprimirCupom,
        // cancelar
        cancelarVisible,
        cancelar,
        // consulta de estoque/preço
        consultaVisible,
        consultaTermo,
        resultadosConsulta,
        abrirConsulta,
        stockByProduct,
        // teclado
        atalhos,
        // utilitários da view
        formatCentavos,
    })
}
