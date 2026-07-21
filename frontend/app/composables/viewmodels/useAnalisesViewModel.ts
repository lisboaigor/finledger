import type { ChartConfig } from '@/components/ui/chart'
import type { ComercialBi, EstoqueBi, FinanceiroBi } from '~/models/bi'
import { obterComercialBi, obterEstoqueBi, obterFinanceiroBi } from '~/models/bi'

/** ViewModel da página de Análises (BI V2): Financeiro/Caixa, Comercial e
 * Estoque & Compras. Somente leitura — as ações apontam para as telas
 * operacionais (Financeiro, Orçamentos, Compras). */
export function useAnalisesViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { notifyError } = useNotify()

    const loading = ref(true)
    const financeiro = ref<FinanceiroBi | null>(null)
    const comercial = ref<ComercialBi | null>(null)
    const estoque = ref<EstoqueBi | null>(null)

    async function carregar() {
        loading.value = true
        try {
            const [f, c, e] = await Promise.all([
                obterFinanceiroBi(apiFetch),
                obterComercialBi(apiFetch),
                obterEstoqueBi(apiFetch),
            ])
            financeiro.value = f
            comercial.value = c
            estoque.value = e
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    // ── Financeiro ────────────────────────────────────────────────────────────

    /** Decomposição do ciclo de caixa em linguagem simples — cada card explica
     * o que fazer, não só o número. Pensado para quem nunca ouviu falar em
     * "DSO/DIO/DPO/CCC": esses termos técnicos aparecem só entre parênteses. */
    const cicloCards = computed(() => {
        const c = financeiro.value?.ciclo
        if (!c) return []
        return [
            {
                label: 'Tempo para receber dos clientes',
                dias: c.dso,
                meta: c.dso < 30 ? 'bom: até 30 dias' : 'o ideal é até 30 dias',
                tom: c.dso < 30 ? 'bom' : c.dso <= 45 ? 'atencao' : 'critico',
            },
            {
                label: 'Tempo que o produto fica parado até vender',
                dias: c.dio,
                meta: c.dio <= 60 ? 'bom: até 60 dias' : 'o ideal é até 60 dias',
                tom: c.dio <= 60 ? 'bom' : c.dio <= 90 ? 'atencao' : 'critico',
            },
            {
                label: 'Tempo que você tem para pagar fornecedores',
                dias: c.dpo,
                meta: c.dpo >= c.dso ? 'bom: maior que o tempo de receber' : 'tente negociar mais prazo com fornecedores',
                tom: c.dpo >= c.dso ? 'bom' : 'atencao',
            },
            {
                label: 'Quantos dias seu dinheiro fica parado',
                dias: c.ccc,
                meta: c.ccc < 30 ? 'bom: até 30 dias' : 'quanto menor, melhor',
                tom: c.ccc < 30 ? 'bom' : c.ccc <= 60 ? 'atencao' : 'critico',
            },
        ]
    })

    /** Texto de apoio acima dos 4 cards, explicando o que eles significam juntos. */
    const explicacaoCiclo = computed(() => {
        const c = financeiro.value?.ciclo
        if (!c) return ''
        return `No total, o seu dinheiro fica ${Math.round(c.ccc)} dia(s) "preso" no negócio: primeiro parado no estoque, depois esperando o cliente pagar — e nesse tempo todo você já pagou (ou vai pagar) os fornecedores. Quanto menor esse número, mais rápido o dinheiro volta para o seu bolso.`
    })

    const chartAgingData = computed(() =>
        (financeiro.value?.aging ?? []).map((a) => ({
            faixa: a.faixa,
            saldo: a.total_centavos / 100,
            fill: a.faixa === 'A vencer' ? '#1AA886' : a.faixa.startsWith('1–30') ? '#f59e0b' : '#ef4444',
        })),
    )

    const chartAgingConfig = {
        saldo: { label: 'Saldo' },
    } satisfies ChartConfig

    const chartProjecaoData = computed(() =>
        (financeiro.value?.projecao ?? []).map((s) => ({
            semana: s.semana,
            receber: s.receber_centavos / 100,
            pagar: s.pagar_centavos / 100,
        })),
    )

    const chartProjecaoConfig = {
        receber: { label: 'A receber', color: '#1AA886' },
        pagar: { label: 'A pagar', color: '#ef4444' },
    } satisfies ChartConfig

    // ── Comercial ─────────────────────────────────────────────────────────────

    const ordemFunil = ['rascunho', 'emitido', 'aceito', 'convertido', 'recusado', 'expirado', 'cancelado']
    const funilOrdenado = computed(() =>
        [...(comercial.value?.funil ?? [])].sort(
            (a, b) => ordemFunil.indexOf(a.status) - ordemFunil.indexOf(b.status),
        ),
    )

    /** Conversão do funil 90d: convertidos ÷ decididos. */
    const conversaoFunil = computed(() => {
        const f = comercial.value?.funil ?? []
        const conta = (s: string) => f.find((x) => x.status === s)?.quantidade ?? 0
        const decididos = conta('aceito') + conta('recusado') + conta('expirado') + conta('convertido')
        return decididos > 0 ? (conta('convertido') / decididos) * 100 : null
    })

    const segmentoSeverity: Record<string, string> = {
        'Campeão': 'success',
        'Fiel': 'info',
        'Novo/Recente': 'info',
        'Em risco': 'warn',
        'Perdido': 'danger',
        'Ocasional': 'secondary',
    }

    // ── Estoque ───────────────────────────────────────────────────────────────

    /** Matriz 3×3 ABC (linhas) × XYZ (colunas) pronta para o grid. */
    const matriz = computed(() => {
        const celulas = estoque.value?.matriz ?? []
        const get = (abc: string, xyz: string) => celulas.find((c) => c.abc === abc && c.xyz === xyz)
        return ['A', 'B', 'C'].map((abc) => ({
            abc,
            colunas: ['X', 'Y', 'Z', '—'].map((xyz) => ({ xyz, celula: get(abc, xyz) ?? null })),
        }))
    })

    function linkPedido(r: { produto_id: string; sugestao_compra: number }) {
        return `/compras?produto=${r.produto_id}&quantidade=${r.sugestao_compra}`
    }

    return reactive({
        loading,
        carregar,
        financeiro,
        comercial,
        estoque,
        cicloCards,
        explicacaoCiclo,
        chartAgingData,
        chartAgingConfig,
        chartProjecaoData,
        chartProjecaoConfig,
        funilOrdenado,
        conversaoFunil,
        segmentoSeverity,
        matriz,
        linkPedido,
    })
}
