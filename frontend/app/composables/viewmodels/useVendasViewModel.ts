import type { Opcao } from '~/models/shared'
import type { Produto } from '~/models/catalogo'
import { listarProdutos } from '~/models/catalogo'
import type { Cliente } from '~/models/crm'
import { listarClientes } from '~/models/crm'
import type { Saldo } from '~/models/estoque'
import { listarSaldos } from '~/models/estoque'
import type { Venda, VendaDetalhes, VendaItem } from '~/models/vendas'
import {
    adicionarItemVenda,
    atualizarClienteVenda,
    buscarVenda,
    cancelarVenda,
    confirmarVenda,
    definirFormaPagamento,
    devolverItensVenda,
    iniciarVenda,
    listarVendas,
    removerItemVenda,
} from '~/models/vendas'

/** ViewModel da página de Vendas: concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useVendasViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { notifySuccess, notifyError } = useNotify()

    // --- Listagem ---
    const vendas = ref<Venda[]>([])
    const produtos = ref<Produto[]>([])
    const clientes = ref<Cliente[]>([])
    const saldos = ref<Saldo[]>([])
    const loading = ref(false)

    /** Caixa de busca única: cliente/status/pagamento casam no navegador
     * (fuzzy, sobre `vendas` já carregado); produto casa no backend — os
     * itens de cada venda não ficam carregados no navegador (histórico
     * cresce sem limite, ao contrário do catálogo). O resultado exibido é a
     * união dos dois, então uma venda aparece se casar por qualquer um dos
     * critérios. */
    const busca = ref('')
    const vendasPorProduto = ref<Venda[]>([])
    let debounceBusca: ReturnType<typeof setTimeout> | null = null

    function onBuscaChange() {
        if (debounceBusca) clearTimeout(debounceBusca)
        debounceBusca = setTimeout(() => {
            void buscarPorProduto()
        }, 350)
    }

    async function buscarPorProduto() {
        const termo = busca.value.trim()
        if (!termo) {
            vendasPorProduto.value = []
            return
        }
        try {
            const { vendas: v } = await listarVendas(apiFetch, { produtoBusca: termo })
            vendasPorProduto.value = v
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    const saldoPorProduto = computed(() => {
        const m = new Map<string, number>()
        saldos.value.forEach((s) => m.set(s.produto_id, s.quantidade))
        return m
    })

    const clientePorId = computed(() => {
        const m = new Map<string, string>()
        clientes.value.forEach((c) => m.set(c.cliente_id, c.nome))
        return m
    })

    function nomeCliente(id: string | null) {
        return id ? (clientePorId.value.get(id) ?? '—') : 'Consumidor'
    }

    /** Nome + telefone/documento numa linha, para uso em impressões. */
    function clienteResumo(id: string | null): string {
        if (!id) return 'Consumidor'
        const c = clientes.value.find((x) => x.cliente_id === id)
        if (!c) return '—'
        const contato = c.telefone || c.cpf_cnpj
        return contato ? `${c.nome} (${contato})` : c.nome
    }

    function statusSeverity(status: string) {
        return { Confirmada: 'success', EmAndamento: 'info', Cancelada: 'danger' }[status] ?? 'secondary'
    }

    const { buscarAproximado } = useFuzzySearch()

    const vendasFiltradas = computed(() => {
        const porTexto = buscarAproximado(
            vendas.value,
            busca.value,
            (v) => `${nomeCliente(v.cliente_id)} ${v.status} ${v.forma_pagamento ?? ''}`,
        )
        if (!busca.value.trim()) return porTexto
        const idsExistentes = new Set(porTexto.map((v) => v.venda_id))
        const porProduto = vendasPorProduto.value.filter((v) => !idsExistentes.has(v.venda_id))
        return [...porTexto, ...porProduto]
    })

    async function carregar() {
        loading.value = true
        try {
            const [{ vendas: v }, { produtos: p }, { clientes: c }, { saldos: s }] = await Promise.all([
                listarVendas(apiFetch),
                listarProdutos(apiFetch),
                listarClientes(apiFetch),
                listarSaldos(apiFetch),
            ])
            vendas.value = v
            produtos.value = p
            clientes.value = c
            saldos.value = s
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    /** Mostra o saldo em estoque no rótulo — o vendedor precisa ter ciência
     * da disponibilidade antes de adicionar o item (produtos que não
     * controlam estoque, como serviços, não mostram saldo). */
    const opcoesProduto = computed<Opcao[]>(() =>
        produtos.value
            .filter((p) => p.ativo)
            .map((p) => ({
                label: p.controla_estoque
                    ? `${p.sku} — ${p.descricao} · ${saldoPorProduto.value.get(p.produto_id) ?? 0} em estoque`
                    : `${p.sku} — ${p.descricao} · serviço`,
                value: p.produto_id,
            })),
    )
    const opcoesCliente = computed<Opcao[]>(() =>
        clientes.value.map((c) => ({ label: c.nome, value: c.cliente_id })),
    )

    // --- Nova venda ---
    const novaVisible = ref(false)
    const novoCliente = ref<string | null>(null)

    async function iniciar() {
        try {
            const { venda_id } = await iniciarVenda(apiFetch, novoCliente.value)
            novaVisible.value = false
            novoCliente.value = null
            await carregar()
            await abrirDetalhe(venda_id)
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Detalhe ---
    const detalheVisible = ref(false)
    const detalhe = ref<VendaDetalhes | null>(null)
    const carregandoDetalhe = ref(false)

    async function buscarDetalheAtual(id: string) {
        try {
            detalhe.value = await buscarVenda(apiFetch, id)
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    const { printReceipt } = useThermalPrint()
    const { tenantSlug } = useAuth()
    const { businessInfo, garantirCarregado: garantirEmpresaCarregada } = useEmpresaInfo()
    void garantirEmpresaCarregada()

    /** Recibo da venda em aberto no detalhe (não fiscal) — mesmo formato do PDV. */
    function imprimirVenda() {
        const d = detalhe.value
        if (!d) return
        printReceipt({
            storeName: tenantSlug.value || 'Finledger',
            title: 'RECIBO DE VENDA',
            reference: `Nº ${d.venda.venda_id.slice(0, 8)} · ${d.venda.status}`,
            meta: [{ label: 'Cliente', value: clienteResumo(d.venda.cliente_id) }],
            businessInfo: businessInfo.value,
            items: d.itens.map((i) => ({
                descricao: i.descricao,
                sku: i.sku,
                quantidade: i.quantidade,
                unitCents: i.preco_unitario_centavos,
            })),
            totalCents: d.venda.total_centavos,
            paymentLabel: d.venda.forma_pagamento ? `Pagamento: ${d.venda.forma_pagamento}` : undefined,
            footerNote: 'Recibo sem valor fiscal.',
        })
    }

    async function abrirDetalhe(id: string) {
        detalheVisible.value = true
        carregandoDetalhe.value = true
        await buscarDetalheAtual(id)
        carregandoDetalhe.value = false
    }

    async function recarregarDetalhe() {
        if (detalhe.value) await buscarDetalheAtual(detalhe.value.venda.venda_id)
        await carregar()
    }

    const emAndamento = computed(() => detalhe.value?.venda.status === 'Em Andamento')

    function podeEditarOuExcluir(venda: Venda) {
        return venda.status === 'Em Andamento'
    }

    // editar cliente da venda
    const editandoCliente = ref<string | null>(null)
    const salvandoCliente = ref(false)

    async function salvarCliente() {
        if (!detalhe.value) return
        salvandoCliente.value = true
        try {
            await atualizarClienteVenda(apiFetch, detalhe.value.venda.venda_id, editandoCliente.value)
            notifySuccess('Cliente atualizado', undefined, 2500)
            await recarregarDetalhe()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoCliente.value = false
        }
    }

    watch(detalhe, (d) => {
        editandoCliente.value = d?.venda.cliente_id ?? null
    })

    // adicionar item
    const novoItem = reactive({
        produto_id: null as string | null,
        quantidade: 1,
        vender_sem_estoque: false,
    })

    /** Produto escolhido controla estoque e a quantidade pedida excede o
     * saldo — mostra a opção de vender sob encomenda na tela. */
    const estoqueInsuficiente = computed(() => {
        const p = novoItem.produto_id ? produtos.value.find((x) => x.produto_id === novoItem.produto_id) : null
        if (!p || !p.controla_estoque) return false
        return (saldoPorProduto.value.get(p.produto_id) ?? 0) < novoItem.quantidade
    })

    async function adicionarItemAtual() {
        if (!detalhe.value || !novoItem.produto_id) return
        const p = produtos.value.find((x) => x.produto_id === novoItem.produto_id)
        if (!p) return
        try {
            await adicionarItemVenda(apiFetch, detalhe.value.venda.venda_id, {
                produto_id: p.produto_id,
                sku: p.sku,
                descricao: p.descricao,
                quantidade: novoItem.quantidade,
                preco_unitario_centavos: p.preco_venda,
                vender_sem_estoque: novoItem.vender_sem_estoque,
            })
            novoItem.produto_id = null
            novoItem.quantidade = 1
            novoItem.vender_sem_estoque = false
            await recarregarDetalhe()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    async function removerItemAtual(item: VendaItem) {
        if (!detalhe.value) return
        try {
            await removerItemVenda(apiFetch, detalhe.value.venda.venda_id, item.item_id)
            await recarregarDetalhe()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // forma de pagamento
    const formaTipo = ref('Dinheiro')
    const parcelas = ref(2)
    const prazoDias = ref(30)
    const formasOpcoes = [
        { label: 'Dinheiro', value: 'Dinheiro' },
        { label: 'Cartão de Débito', value: 'CartaoDebito' },
        { label: 'Cartão de Crédito', value: 'CartaoCredito' },
        { label: 'Pix', value: 'Pix' },
        { label: 'A prazo', value: 'Prazo' },
    ]

    function montarForma() {
        switch (formaTipo.value) {
            case 'CartaoCredito':
                return { CartaoCredito: { parcelas: parcelas.value } }
            case 'Prazo':
                return { Prazo: { dias: prazoDias.value } }
            default:
                return formaTipo.value
        }
    }

    async function definirPagamento() {
        if (!detalhe.value) return
        try {
            await definirFormaPagamento(apiFetch, detalhe.value.venda.venda_id, montarForma())
            notifySuccess('OK', 'Forma de pagamento definida.', 2500)
            await recarregarDetalhe()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    async function confirmar() {
        if (!detalhe.value) return
        try {
            await confirmarVenda(apiFetch, detalhe.value.venda.venda_id)
            notifySuccess('Venda confirmada')
            await recarregarDetalhe()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Devolução de itens ---
    const devolverVisible = ref(false)
    const motivoDevolucao = ref('')
    const salvandoDevolucao = ref(false)
    const itensDevolucao = ref<
        { item_id: string; sku: string; descricao: string; vendida: number; quantidade: number }[]
    >([])

    const vendaConfirmada = computed(() => detalhe.value?.venda.status === 'Confirmada')

    function abrirDevolucao() {
        if (!detalhe.value) return
        motivoDevolucao.value = ''
        itensDevolucao.value = detalhe.value.itens.map((i) => ({
            item_id: i.item_id,
            sku: i.sku,
            descricao: i.descricao,
            vendida: i.quantidade,
            quantidade: 0,
        }))
        devolverVisible.value = true
    }

    const devolucaoTotal = computed(
        () =>
            itensDevolucao.value.length > 0 &&
            itensDevolucao.value.every((i) => i.quantidade >= i.vendida),
    )
    const devolucaoValida = computed(
        () =>
            motivoDevolucao.value.trim().length > 0 &&
            itensDevolucao.value.some((i) => i.quantidade > 0) &&
            itensDevolucao.value.every((i) => i.quantidade >= 0 && i.quantidade <= i.vendida),
    )

    async function confirmarDevolucao() {
        if (!detalhe.value || !devolucaoValida.value) return
        salvandoDevolucao.value = true
        try {
            await devolverItensVenda(
                apiFetch,
                detalhe.value.venda.venda_id,
                itensDevolucao.value
                    .filter((i) => i.quantidade > 0)
                    .map((i) => ({ item_id: i.item_id, quantidade: i.quantidade })),
                motivoDevolucao.value,
            )
            notifySuccess(
                devolucaoTotal.value ? 'Devolução total registrada' : 'Devolução registrada',
                devolucaoTotal.value
                    ? 'Venda desfeita, estoque reposto e cancelamento da NF encaminhado.'
                    : 'Estoque reposto; a NF será reemitida com os itens restantes quando a integração SEFAZ estiver ativa.',
            )
            devolverVisible.value = false
            await recarregarDetalhe()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoDevolucao.value = false
        }
    }

    const cancelarVisible = ref(false)
    const motivoCancelamento = ref('')
    const vendaParaExcluir = ref<Venda | null>(null)

    function abrirExclusao(venda: Venda) {
        vendaParaExcluir.value = venda
        motivoCancelamento.value = ''
        cancelarVisible.value = true
    }

    async function cancelarAtual() {
        const alvo = vendaParaExcluir.value ?? detalhe.value?.venda
        if (!alvo) return
        try {
            await cancelarVenda(apiFetch, alvo.venda_id, motivoCancelamento.value)
            notifySuccess('Venda excluída', 'Venda cancelada com sucesso.')
            cancelarVisible.value = false
            motivoCancelamento.value = ''
            vendaParaExcluir.value = null
            if (detalhe.value?.venda.venda_id === alvo.venda_id) await recarregarDetalhe()
            else await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    return reactive({
        vendas,
        produtos,
        clientes,
        loading,
        busca,
        onBuscaChange,
        nomeCliente,
        statusSeverity,
        vendasFiltradas,
        carregar,
        opcoesProduto,
        opcoesCliente,
        novaVisible,
        novoCliente,
        iniciarVenda: iniciar,
        detalheVisible,
        detalhe,
        carregandoDetalhe,
        imprimirVenda,
        abrirDetalhe,
        recarregarDetalhe,
        emAndamento,
        podeEditarOuExcluir,
        editandoCliente,
        salvandoCliente,
        salvarCliente,
        novoItem,
        estoqueInsuficiente,
        adicionarItem: adicionarItemAtual,
        removerItem: removerItemAtual,
        formaTipo,
        parcelas,
        prazoDias,
        formasOpcoes,
        definirPagamento,
        confirmarVenda: confirmar,
        cancelarVisible,
        motivoCancelamento,
        vendaParaExcluir,
        abrirExclusao,
        cancelarVenda: cancelarAtual,
        devolverVisible,
        motivoDevolucao,
        salvandoDevolucao,
        itensDevolucao,
        vendaConfirmada,
        abrirDevolucao,
        devolucaoTotal,
        devolucaoValida,
        confirmarDevolucao,
    })
}
