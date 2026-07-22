import type { ChartConfig } from '@/components/ui/chart'
import type { BiResumo, ReceitaDia, SaudeNegocio } from '~/models/bi'
import { obterResumoBi } from '~/models/bi'

/** ViewModel do BI prescritivo no dashboard "Hoje": KPIs com faixas de leitura
 * (bom/atenção/crítico). Os alertas/recomendações vivem em useNotificacoes
 * (compartilhados com o sino do topbar). */
export function useBiViewModel() {
    const { apiFetch } = useApi()
    const { formatCentavos } = useFormat()

    const loading = ref(true)
    const resumo = ref<BiResumo | null>(null)
    const receitaDiaria = ref<ReceitaDia[]>([])
    const saude = ref<SaudeNegocio | null>(null)
    const metaCentavos = ref<number | null>(null)

    async function carregar() {
        loading.value = true
        try {
            const r = await obterResumoBi(apiFetch)
            resumo.value = r.resumo
            receitaDiaria.value = r.receita_diaria
            saude.value = r.saude
            metaCentavos.value = r.meta_faturamento_mensal_centavos
        } catch {
            /* módulo de BI indisponível — o dashboard operacional continua */
        } finally {
            loading.value = false
        }
    }

    type Tom = 'bom' | 'atencao' | 'critico' | 'neutro'

    /** Variação da receita vs mês anterior (%), quando há base de comparação. */
    const variacaoReceita = computed(() => {
        const r = resumo.value
        if (!r || r.receita_mes_anterior_centavos <= 0) return null
        return ((r.receita_mes_centavos - r.receita_mes_anterior_centavos) / r.receita_mes_anterior_centavos) * 100
    })

    const indicadores = computed(() => {
        const r = resumo.value
        if (!r) return []
        const variacao = variacaoReceita.value
        const margem = r.margem_percent
        const itens: { label: string; value: string; detalhe: string; tom: Tom; icon: string }[] = [
            {
                label: 'Quanto você vendeu este mês',
                value: formatCentavos(r.receita_mes_centavos),
                detalhe:
                    variacao === null
                        ? 'ainda não dá para comparar com o mês passado'
                        : variacao >= 0
                          ? `vendeu ${variacao.toFixed(0)}% a mais que no mês passado`
                          : `vendeu ${Math.abs(variacao).toFixed(0)}% a menos que no mês passado`,
                tom: variacao === null ? 'neutro' : variacao >= 0 ? 'bom' : variacao >= -10 ? 'atencao' : 'critico',
                icon: 'pi pi-chart-line',
            },
            {
                label: 'Dinheiro esperado nos próximos 30 dias',
                value: formatCentavos(r.caixa_30d_centavos),
                detalhe:
                    r.caixa_30d_centavos >= 0
                        ? 'depois de pagar tudo, ainda sobra dinheiro'
                        : 'o que você precisa pagar é maior que o que vão te pagar',
                tom: r.caixa_30d_centavos >= 0 ? 'bom' : 'critico',
                icon: 'pi pi-wallet',
            },
            {
                label: 'Dinheiro atrasado para receber',
                value: formatCentavos(r.vencidas_centavos),
                detalhe: r.vencidas_centavos > 0 ? 'clientes que já deviam ter pago' : 'ninguém está te devendo 🎉',
                tom: r.vencidas_centavos === 0 ? 'bom' : 'critico',
                icon: 'pi pi-exclamation-circle',
            },
            {
                label: 'Margem de balcão (preço − custo do produto)',
                value: margem === null ? '—' : `R$ ${margem.toFixed(0)} a cada R$ 100 vendidos`,
                detalhe:
                    margem === null
                        ? 'ainda não há vendas este mês para calcular'
                        : margem >= 28
                          ? 'boa arrecadação — daqui ainda saem custos fixos e taxas; a sobra final é menor'
                          : margem >= 20
                            ? 'arrecadando um pouco menos que a referência (R$ 28 a cada R$ 100)'
                            : 'arrecadação baixa — pode não pagar os custos fixos; revise preços ou custos',
                tom: margem === null ? 'neutro' : margem >= 28 ? 'bom' : margem >= 20 ? 'atencao' : 'critico',
                icon: 'pi pi-percentage',
            },
        ]

        // Margem já descontados os impostos que são custo do vendedor (reforma
        // LC 214/2025). Aparece quando há vendas com nota no mês; a diferença
        // para a margem de balcão é o peso dos tributos.
        const liquida = r.margem_liquida_percent
        if (liquida !== null) {
            itens.push({
                label: 'Margem depois dos impostos',
                value: `R$ ${liquida.toFixed(0)} a cada R$ 100 vendidos`,
                detalhe:
                    margem !== null && margem > liquida
                        ? `os impostos consomem cerca de R$ ${(margem - liquida).toFixed(0)} a cada R$ 100`
                        : 'sobra após os impostos que são custo da loja',
                tom: liquida >= 20 ? 'bom' : liquida >= 12 ? 'atencao' : 'critico',
                icon: 'pi pi-receipt',
            })
        }
        return itens
    })

    const chartReceitaData = computed(() =>
        receitaDiaria.value.map((d) => {
            const [, m, dia] = d.dia.split('-')
            return { dia: `${dia}/${m}`, receita: d.total_centavos / 100 }
        }),
    )

    const chartReceitaConfig = {
        receita: { label: 'Receita', color: 'var(--chart-1)' },
    } satisfies ChartConfig

    /** Progresso da meta de faturamento do mês, com o ritmo necessário para
     * fechá-la (por dia útil restante, seg–sáb). */
    const metaProgresso = computed(() => {
        const meta = metaCentavos.value
        const realizado = resumo.value?.receita_mes_centavos
        if (!meta || meta <= 0 || realizado == null) return null

        const hoje = new Date()
        const fimDoMes = new Date(hoje.getFullYear(), hoje.getMonth() + 1, 0)
        let diasUteisRestantes = 0
        for (let d = hoje.getDate(); d <= fimDoMes.getDate(); d++) {
            const dia = new Date(hoje.getFullYear(), hoje.getMonth(), d).getDay()
            if (dia !== 0) diasUteisRestantes++ // exclui domingos
        }

        const pct = Math.min(100, (100 * realizado) / meta)
        const falta = Math.max(0, meta - realizado)
        return {
            meta,
            realizado,
            pct,
            falta,
            porDiaUtil: diasUteisRestantes > 0 ? Math.ceil(falta / diasUteisRestantes) : falta,
            diasUteisRestantes,
            atingida: realizado >= meta,
        }
    })

    /** Leitura do score: faixa de cor e frase-resumo. */
    const tomSaude = computed(() => {
        const s = saude.value?.score
        if (s == null) return null
        if (s >= 80) return { tom: 'bom', frase: 'Negócio saudável' }
        if (s >= 60) return { tom: 'atencao', frase: 'Alguns pontos pedem atenção' }
        return { tom: 'critico', frase: 'Pontos críticos para resolver' }
    })

    return reactive({
        loading,
        carregar,
        resumo,
        saude,
        tomSaude,
        metaProgresso,
        indicadores,
        chartReceitaData,
        chartReceitaConfig,
        temDadosReceita: computed(() => receitaDiaria.value.some((d) => d.total_centavos > 0)),
    })
}
