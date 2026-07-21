import type { Produto } from '~/models/catalogo'
import { listarProdutos } from '~/models/catalogo'
import type { SugestaoPreco } from '~/composables/useMargens'

export interface LinhaAnalisePreco {
    produto: Produto
    /** max(custo do cadastro, custo médio do estoque) — base da sugestão. */
    custoBaseCentavos: number
    sugestao: SugestaoPreco
    /** Margem líquida no preço atual (% do preço); null sem preço. */
    margemAtualPct: number | null
    /** Sugerido − atual, em centavos (positivo = está barato demais). */
    deltaCentavos: number
    saldo: number
    /** Ganho extra se vender o saldo atual ao preço sugerido (só deltas > 0). */
    ganhoPotencialCentavos: number
    encalhado: boolean
}

/** Aba "Preços e Margens" do BI: compara o preço praticado de cada produto com
 * a sugestão do assistente (mesmo cálculo de useMargens — nada é refeito aqui)
 * e aponta onde há margem perdida ou estoque encalhado precisando de desconto. */
export function useAnalisePrecosViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { notifyError } = useNotify()
    const { sugerirPreco, lucroLiquido, giroDoProduto, garantirCarregado, config } = useMargens()

    const loading = ref(true)
    const produtos = ref<Produto[]>([])

    async function carregar() {
        loading.value = true
        try {
            const [{ produtos: p }] = await Promise.all([listarProdutos(apiFetch), garantirCarregado()])
            produtos.value = p
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    const configurado = computed(() => config.value.margemPadraoPct != null)

    const linhas = computed<LinhaAnalisePreco[]>(() =>
        produtos.value
            .filter((p) => p.ativo && p.preco_custo > 0)
            .flatMap((p) => {
                const giro = giroDoProduto(p.produto_id)
                // Custo base conservador: o maior entre o cadastro e o custo
                // médio real do estoque (mesmo critério do alerta A7) — se o
                // estoque custou mais caro, a sugestão não pode fingir que não.
                const custoBase = Math.max(p.preco_custo, giro?.custo_medio_centavos ?? 0)
                const s = sugerirPreco(custoBase, p.categoria, p.produto_id)
                if (!s || 'invalido' in s) return []
                const lucro = lucroLiquido(s.custoTotalCentavos, p.preco_venda, p.produto_id, p.categoria)
                const delta = s.precoCentavos - p.preco_venda
                const saldo = giro?.saldo ?? 0
                return [
                    {
                        produto: p,
                        custoBaseCentavos: custoBase,
                        sugestao: s,
                        margemAtualPct: lucro?.percentual ?? null,
                        deltaCentavos: delta,
                        saldo,
                        ganhoPotencialCentavos: delta > 0 ? delta * saldo : 0,
                        encalhado: (s.ajusteGiro?.pontos ?? 0) < 0,
                    },
                ]
            })
            // Maior distância entre praticado e sugerido primeiro (em % do preço).
            .sort(
                (a, b) =>
                    Math.abs(b.deltaCentavos) / (b.produto.preco_venda || 1) -
                    Math.abs(a.deltaCentavos) / (a.produto.preco_venda || 1),
            ),
    )

    /** Abaixo do alvo com folga (> 2 p.p.) — tolerância para arredondamentos. */
    const abaixoDoAlvo = computed(() =>
        linhas.value.filter((l) => l.margemAtualPct != null && l.margemAtualPct < l.sugestao.margemPct - 2),
    )
    const encalhados = computed(() => linhas.value.filter((l) => l.encalhado))
    const ganhoPotencialTotal = computed(() =>
        abaixoDoAlvo.value.reduce((s, l) => s + l.ganhoPotencialCentavos, 0),
    )

    // Passo a passo por produto (dialog)
    const detalhe = ref<LinhaAnalisePreco | null>(null)

    return reactive({
        loading,
        carregar,
        configurado,
        linhas,
        abaixoDoAlvo,
        encalhados,
        ganhoPotencialTotal,
        detalhe,
    })
}
