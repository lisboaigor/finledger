import type { ChartConfig } from '@/components/ui/chart'
import type {
    ClienteResumo,
    NotaResumo,
    OrcamentoResumo,
    ProdutoResumo,
    SaldoResumo,
    VendaResumo,
} from '~/models/dashboard'
import {
    listarClientesResumo,
    listarContasPagarResumo,
    listarContasReceberResumo,
    listarNotasResumo,
    listarOrcamentosResumo,
    listarProdutosResumo,
    listarSaldosResumo,
    listarVendasResumo,
} from '~/models/dashboard'

export interface AtividadeItem {
    icon: string
    color: string
    titulo: string
    descricao: string
    to: string
}

/** ViewModel do Dashboard: agrega resumos de vários módulos (leitura, sem regras
 * de escrita) e monta os cards/gráficos/listas exibidos na página inicial. */
export function useDashboardViewModel() {
    const { apiFetch } = useApi()
    const { username } = useAuth()
    const { formatCentavos, formatNumber } = useFormat()

    const loading = ref(true)
    const stats = reactive({
        produtos: 0,
        clientes: 0,
        vendasConfirmadas: 0,
        faturamento: 0,
        orcamentosAbertos: 0,
        aReceber: 0,
        aPagar: 0,
        notas: 0,
    })

    const vendasRaw = ref<VendaResumo[]>([])
    const orcamentosRaw = ref<OrcamentoResumo[]>([])
    const notasRaw = ref<NotaResumo[]>([])
    const saldosRaw = ref<SaldoResumo[]>([])
    const produtosRaw = ref<ProdutoResumo[]>([])
    const clientesRaw = ref<ClienteResumo[]>([])

    /** Faz a chamada e ignora erros (ex.: 403 por papel sem acesso). */
    async function safe<T>(fn: () => Promise<T>, onOk: (data: T) => void) {
        try {
            onOk(await fn())
        } catch {
            /* sem acesso — mantém zero */
        }
    }

    async function carregar() {
        await Promise.all([
            safe(
                () => listarProdutosResumo(apiFetch),
                (d) => {
                    produtosRaw.value = d.produtos
                    stats.produtos = d.produtos.filter((p) => p.ativo).length
                },
            ),
            safe(
                () => listarClientesResumo(apiFetch),
                (d) => {
                    clientesRaw.value = d.clientes
                    stats.clientes = d.clientes.length
                },
            ),
            safe(
                () => listarVendasResumo(apiFetch),
                (d) => {
                    vendasRaw.value = d.vendas
                    const confirmadas = d.vendas.filter((v) => v.status === 'Confirmada')
                    stats.vendasConfirmadas = confirmadas.length
                    stats.faturamento = confirmadas.reduce((s, v) => s + v.total_centavos, 0)
                },
            ),
            safe(
                () => listarOrcamentosResumo(apiFetch),
                (d) => {
                    orcamentosRaw.value = d.orcamentos
                    stats.orcamentosAbertos = d.orcamentos.filter((o) =>
                        ['Rascunho', 'Emitido'].includes(o.status),
                    ).length
                },
            ),
            safe(
                () => listarContasReceberResumo(apiFetch),
                (d) =>
                    (stats.aReceber = d.contas.reduce(
                        (s, c) => s + (c.valor_original - (c.valor_recebido ?? 0)),
                        0,
                    )),
            ),
            safe(
                () => listarContasPagarResumo(apiFetch),
                (d) => (stats.aPagar = d.contas.reduce((s, c) => s + (c.valor_original - (c.valor_pago ?? 0)), 0)),
            ),
            safe(
                () => listarSaldosResumo(apiFetch),
                (d) => (saldosRaw.value = d.saldos),
            ),
            safe(
                () => listarNotasResumo(apiFetch),
                (d) => {
                    notasRaw.value = d.notas
                    stats.notas = d.notas.length
                },
            ),
        ])
        loading.value = false
    }

    const cards = computed(() => [
        { label: 'Faturamento', value: formatCentavos(stats.faturamento), icon: 'pi pi-chart-line', color: 'text-green-500', bg: 'bg-green-100 dark:bg-green-400/10', to: '/vendas' },
        { label: 'A receber', value: formatCentavos(stats.aReceber), icon: 'pi pi-arrow-down-left', color: 'text-blue-500', bg: 'bg-blue-100 dark:bg-blue-400/10', to: '/financeiro' },
        { label: 'A pagar', value: formatCentavos(stats.aPagar), icon: 'pi pi-arrow-up-right', color: 'text-red-500', bg: 'bg-red-100 dark:bg-red-400/10', to: '/financeiro' },
        { label: 'Vendas confirmadas', value: formatNumber(stats.vendasConfirmadas), icon: 'pi pi-shopping-cart', color: 'text-primary', bg: 'bg-primary/10', to: '/vendas' },
        { label: 'Orçamentos em aberto', value: formatNumber(stats.orcamentosAbertos), icon: 'pi pi-file', color: 'text-orange-500', bg: 'bg-orange-100 dark:bg-orange-400/10', to: '/orcamentos' },
        { label: 'Produtos ativos', value: formatNumber(stats.produtos), icon: 'pi pi-tag', color: 'text-purple-500', bg: 'bg-purple-100 dark:bg-purple-400/10', to: '/catalogo' },
        { label: 'Clientes', value: formatNumber(stats.clientes), icon: 'pi pi-users', color: 'text-cyan-500', bg: 'bg-cyan-100 dark:bg-cyan-400/10', to: '/clientes' },
        { label: 'Notas fiscais', value: formatNumber(stats.notas), icon: 'pi pi-file-check', color: 'text-teal-500', bg: 'bg-teal-100 dark:bg-teal-400/10', to: '/fiscal' },
    ])

    const clientePorId = computed(() => {
        const m = new Map<string, string>()
        clientesRaw.value.forEach((c) => m.set(c.cliente_id, c.nome))
        return m
    })
    function nomeCliente(id: string | null) {
        return id ? (clientePorId.value.get(id) ?? '—') : 'Consumidor'
    }

    const estoqueBaixo = computed(() => {
        const produtoPorId = new Map(produtosRaw.value.map((p) => [p.produto_id, p]))
        return saldosRaw.value
            .filter((s) => s.estoque_minimo > 0 && s.quantidade <= s.estoque_minimo)
            .map((s) => ({
                ...s,
                produto: produtoPorId.get(s.produto_id),
                percentual: Math.min(100, Math.round((s.quantidade / s.estoque_minimo) * 100)),
            }))
            .sort((a, b) => a.percentual - b.percentual)
            .slice(0, 5)
    })

    const atividadeRecente = computed<AtividadeItem[]>(() => {
        const itens: AtividadeItem[] = []
        vendasRaw.value.slice(0, 3).forEach((v) =>
            itens.push({
                icon: 'pi pi-shopping-cart',
                color: 'bg-primary',
                titulo: `Venda — ${nomeCliente(v.cliente_id)}`,
                descricao: `${formatCentavos(v.total_centavos)} · ${v.status}`,
                to: '/vendas',
            }),
        )
        orcamentosRaw.value.slice(0, 2).forEach((o) =>
            itens.push({
                icon: 'pi pi-file',
                color: 'bg-orange-500',
                titulo: `Orçamento — ${nomeCliente(o.cliente_id)}`,
                descricao: `${formatCentavos(o.total_centavos)} · ${o.status}`,
                to: '/orcamentos',
            }),
        )
        notasRaw.value.slice(0, 2).forEach((n) =>
            itens.push({
                icon: 'pi pi-file-check',
                color: 'bg-teal-500',
                titulo: `NF-e nº ${n.numero}`,
                descricao: `${formatCentavos(n.total_centavos)} · ${n.status}`,
                to: '/fiscal',
            }),
        )
        return itens
    })

    const chartVendasStatusData = computed(() => {
        const contagem = { Confirmada: 0, EmAndamento: 0, Cancelada: 0 }
        vendasRaw.value.forEach((v) => {
            if (v.status in contagem) contagem[v.status as keyof typeof contagem]++
        })
        return [
            { status: 'Confirmadas', quantidade: contagem.Confirmada, fill: '#22c55e' },
            { status: 'Em andamento', quantidade: contagem.EmAndamento, fill: '#3b82f6' },
            { status: 'Canceladas', quantidade: contagem.Cancelada, fill: '#ef4444' },
        ]
    })

    const chartVendasStatusConfig = {
        quantidade: { label: 'Vendas' },
    } satisfies ChartConfig

    return reactive({
        username,
        loading,
        carregar,
        cards,
        nomeCliente,
        estoqueBaixo,
        atividadeRecente,
        chartVendasStatusData,
        chartVendasStatusConfig,
    })
}
