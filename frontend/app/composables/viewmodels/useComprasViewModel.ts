import type { Opcao } from '~/models/shared'
import type { Produto } from '~/models/catalogo'
import { listarProdutos } from '~/models/catalogo'
import type { Fornecedor } from '~/models/fornecedores'
import { listarFornecedores } from '~/models/fornecedores'
import type { Pedido, PedidoDetalhes } from '~/models/compras'
import {
    aprovarPedido,
    buscarPedido,
    cancelarPedido,
    enviarPedido,
    gerarPedido,
    listarPedidos,
    receberPedido,
} from '~/models/compras'

/** ViewModel da página de Compras: concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useComprasViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { toCentavos } = useFormat()
    const { notifySuccess, notifyError } = useNotify()

    // --- Listagem ---
    const pedidos = ref<Pedido[]>([])
    const produtos = ref<Produto[]>([])
    const fornecedores = ref<Fornecedor[]>([])
    const loading = ref(false)
    const filters = ref({ global: { value: null as string | null, matchMode: 'contains' } })

    const fornecedorPorId = computed(() => {
        const m = new Map<string, string>()
        fornecedores.value.forEach((f) => m.set(f.fornecedor_id, f.razao_social))
        return m
    })
    const fornecedorObjPorId = computed(() => {
        const m = new Map<string, Fornecedor>()
        fornecedores.value.forEach((f) => m.set(f.fornecedor_id, f))
        return m
    })
    const produtoPorId = computed(() => {
        const m = new Map<string, Produto>()
        produtos.value.forEach((p) => m.set(p.produto_id, p))
        return m
    })
    function nomeFornecedor(id: string) {
        return fornecedorPorId.value.get(id) ?? '—'
    }
    function descProduto(id: string) {
        const p = produtoPorId.value.get(id)
        return p ? `${p.sku} — ${p.descricao}` : id
    }
    function statusSeverity(status: string) {
        return (
            {
                Gerado: 'secondary',
                Aprovado: 'info',
                Enviado: 'warn',
                RecebidoParcial: 'warn',
                RecebidoTotal: 'success',
                Cancelado: 'danger',
            }[status] ?? 'secondary'
        )
    }

    async function carregar() {
        loading.value = true
        try {
            const [{ pedidos: pe }, { produtos: pr }, { fornecedores: fo }] = await Promise.all([
                listarPedidos(apiFetch),
                listarProdutos(apiFetch),
                listarFornecedores(apiFetch),
            ])
            pedidos.value = pe
            produtos.value = pr
            fornecedores.value = fo
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    const opcoesProduto = computed<Opcao[]>(() =>
        produtos.value
            .filter((p) => p.ativo)
            .map((p) => ({ label: `${p.sku} — ${p.descricao}`, value: p.produto_id })),
    )
    const opcoesFornecedor = computed<Opcao[]>(() =>
        fornecedores.value.filter((f) => f.ativo).map((f) => ({ label: f.razao_social, value: f.fornecedor_id })),
    )

    // --- Gerar pedido ---
    const gerarVisible = ref(false)
    const salvandoGerar = ref(false)
    const gerar = reactive({
        fornecedor_id: null as string | null,
        prazo_pagamento_dias: 30,
        itens: [] as { produto_id: string | null; quantidade: number; custo: number }[],
    })

    function abrirGerar() {
        gerar.fornecedor_id = null
        gerar.prazo_pagamento_dias = 30
        gerar.itens = [{ produto_id: null, quantidade: 1, custo: 0 }]
        gerarVisible.value = true
    }

    /** Pedido 1-clique: pré-preenche o diálogo a partir de um alerta de ruptura
     * do BI (`/compras?produto=<id>&quantidade=<n>`). */
    function abrirGerarPrefill(produtoId: string, quantidade: number) {
        const p = produtoPorId.value.get(produtoId)
        gerar.fornecedor_id = null
        gerar.prazo_pagamento_dias = 30
        gerar.itens = [
            {
                produto_id: p ? produtoId : null,
                quantidade: Math.max(1, Math.round(quantidade)),
                custo: p ? p.preco_custo / 100 : 0,
            },
        ]
        gerarVisible.value = true
    }
    function adicionarLinha() {
        gerar.itens.push({ produto_id: null, quantidade: 1, custo: 0 })
    }
    function removerLinha(i: number) {
        gerar.itens.splice(i, 1)
    }
    function aoSelecionarProduto(linha: { produto_id: string | null; custo: number }) {
        const p = linha.produto_id ? produtoPorId.value.get(linha.produto_id) : null
        if (p) linha.custo = p.preco_custo / 100
    }

    const gerarValido = computed(
        () =>
            gerar.fornecedor_id &&
            gerar.itens.length > 0 &&
            gerar.itens.every((i) => i.produto_id && i.quantidade > 0),
    )

    async function submeterGerar() {
        if (!gerarValido.value) return
        salvandoGerar.value = true
        try {
            await gerarPedido(apiFetch, {
                fornecedor_id: gerar.fornecedor_id as string,
                prazo_pagamento_dias: gerar.prazo_pagamento_dias,
                itens: gerar.itens.map((i) => ({
                    produto_id: i.produto_id as string,
                    quantidade: i.quantidade,
                    custo_unitario_centavos: toCentavos(i.custo),
                })),
            })
            notifySuccess('Gerado', 'Pedido de compra criado.')
            gerarVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoGerar.value = false
        }
    }

    // --- Detalhe ---
    const detalheVisible = ref(false)
    const detalhe = ref<PedidoDetalhes | null>(null)
    const carregandoDetalhe = ref(false)

    async function abrirDetalhe(id: string) {
        detalheVisible.value = true
        carregandoDetalhe.value = true
        try {
            detalhe.value = await buscarPedido(apiFetch, id)
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            carregandoDetalhe.value = false
        }
    }
    async function recarregar() {
        if (detalhe.value) await abrirDetalhe(detalhe.value.pedido.pedido_id)
        await carregar()
    }

    const st = computed(() => detalhe.value?.pedido.status)

    const { printPurchaseOrderReport } = useThermalPrint()
    const { tenantSlug } = useAuth()

    /** Pedido de compra impresso (A4) — documento interno para conferência/fornecedor. */
    function imprimirPedido() {
        const d = detalhe.value
        if (!d) return
        const fornecedor = fornecedorObjPorId.value.get(d.pedido.fornecedor_id)
        printPurchaseOrderReport({
            storeName: tenantSlug.value || 'Finledger',
            numero: d.pedido.pedido_id.slice(0, 8),
            status: d.pedido.status,
            fornecedorNome: nomeFornecedor(d.pedido.fornecedor_id),
            fornecedorCnpj: fornecedor?.cnpj,
            prazoPagamentoDias: d.pedido.prazo_pagamento_dias,
            items: d.itens.map((i) => {
                const p = produtoPorId.value.get(i.produto_id)
                return {
                    descricao: p?.descricao ?? i.produto_id,
                    sku: p?.sku,
                    quantidade: i.quantidade,
                    unitCents: i.custo_unitario_centavos,
                }
            }),
            totalCents: d.pedido.total_centavos,
        })
    }

    async function aprovar() {
        if (!detalhe.value) return
        try {
            await aprovarPedido(apiFetch, detalhe.value.pedido.pedido_id)
            await recarregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    async function enviar() {
        if (!detalhe.value) return
        try {
            await enviarPedido(apiFetch, detalhe.value.pedido.pedido_id)
            await recarregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // receber
    const receberVisible = ref(false)
    const itensReceber = ref<{ produto_id: string; quantidade: number }[]>([])
    function abrirReceber() {
        if (!detalhe.value) return
        itensReceber.value = detalhe.value.itens.map((i) => ({ produto_id: i.produto_id, quantidade: i.quantidade }))
        receberVisible.value = true
    }
    async function confirmarRecebimento() {
        if (!detalhe.value) return
        try {
            await receberPedido(apiFetch, detalhe.value.pedido.pedido_id, { itens_recebidos: itensReceber.value })
            receberVisible.value = false
            await recarregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // cancelar
    const cancelarVisible = ref(false)
    const motivoCancel = ref('')
    async function cancelar() {
        if (!detalhe.value) return
        try {
            await cancelarPedido(apiFetch, detalhe.value.pedido.pedido_id, motivoCancel.value)
            cancelarVisible.value = false
            motivoCancel.value = ''
            await recarregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    return reactive({
        pedidos,
        produtos,
        fornecedores,
        loading,
        filters,
        nomeFornecedor,
        descProduto,
        statusSeverity,
        carregar,
        opcoesProduto,
        opcoesFornecedor,
        gerarVisible,
        salvandoGerar,
        gerar,
        abrirGerar,
        abrirGerarPrefill,
        adicionarLinha,
        removerLinha,
        aoSelecionarProduto,
        gerarValido,
        submeterGerar,
        detalheVisible,
        detalhe,
        carregandoDetalhe,
        abrirDetalhe,
        recarregar,
        imprimirPedido,
        st,
        aprovar,
        enviar,
        receberVisible,
        itensReceber,
        abrirReceber,
        confirmarRecebimento,
        cancelarVisible,
        motivoCancel,
        cancelar,
    })
}
