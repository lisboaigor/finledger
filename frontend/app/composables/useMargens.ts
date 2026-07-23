import { listarCustosFixos, obterConfiguracoes } from '~/models/configuracoes'
import type { CustoFixo } from '~/models/configuracoes'
import type {
    CategoriaMargem,
    GiroProduto,
    MaquinaCartao,
    MixPagamento,
    Produto,
    ProdutoPrecificacao,
} from '~/models/catalogo'
import {
    listarAliquotasEfetivas,
    listarGiroProdutos,
    listarMaquinasCartao,
    listarMargensCategoria,
    listarPrecificacaoProdutos,
    obterMixPagamento,
} from '~/models/catalogo'

/** Componentes da sugestão de preço — os nomes técnicos (markup sobre o
 * preço, margem líquida) vivem só aqui no código; a UI traduz tudo pra
 * frases simples em R$. */
export interface SugestaoPreco {
    precoCentavos: number
    /** Preço "de prateleira": a conta exata arredondada para cima ao próximo
     * final psicológico (múltiplo de R$ 0,10; a partir de R$ 20, final ,90). */
    precoArredondadoCentavos: number
    custoTotalCentavos: number
    /** Custo DIRETO por unidade além do custo do fornecedor (embalagem,
     * frete-compra rateado) — override explícito por produto/categoria. NÃO é
     * custo fixo de período: entra no custo do item. 0 quando não há override. */
    custoDiretoUnitarioCentavos: number
    origemCustoDireto: 'produto' | 'categoria' | 'padrao' | null
    descontos: { nome: string; pct: number }[]
    /** Margem efetivamente aplicada (base + ajuste de giro). */
    margemPct: number
    /** Margem desejada antes do ajuste de giro. */
    margemBasePct: number
    origemMargem: 'produto' | 'categoria' | 'padrao'
    /** Ajuste por encalhe/volume; null = sem dados ou sem ajuste a fazer. */
    ajusteGiro: AjusteGiro | null
}

/** Ajuste de margem por giro: produto parado perde pontos de margem para
 * girar; produto que vende bem sustenta a margem cheia (pontos = 0, só o
 * argumento). O motivo é exibido como passo do cálculo. */
export interface AjusteGiro {
    pontos: number
    motivo: string
}

export interface SugestaoInvalida {
    invalido: true
    motivo: string
}

interface ConfigPrecificacao {
    margemPadraoPct: number | null
    impostoPct: number
    comissaoPct: number
    cartaoPct: number
    fretePct: number
    outrosPct: number
    custosFixosMensaisCentavos: number | null
    vendasMensaisEstimadas: number | null
    faturamentoMensalCentavos: number | null
}

const bpsParaPct = (bps: number | null) => (bps == null ? null : bps / 100)

/** Preço "de prateleira": arredonda a conta exata PARA CIMA (nunca corrói a
 * margem) até o próximo múltiplo de R$ 0,10; a partir de R$ 20, até o próximo
 * final ",90" (R$ 50,76 → R$ 50,90). */
export function precoPsicologico(centavos: number): number {
    const dezena = Math.ceil(centavos / 10) * 10
    if (dezena < 2000) return dezena
    const resto = dezena % 100
    // X,00 e X,90 já são preços "de prateleira"; o resto sobe até o próximo ,90.
    if (resto === 0 || resto === 90) return dezena
    return resto < 90 ? dezena - resto + 90 : dezena - resto + 190
}

/** Sugestão de preço, lucro por unidade e ponto de equilíbrio — todos os
 * cálculos client-side sobre a configuração de precificação do tenant.
 * Hierarquia de overrides: produto > categoria > padrão da loja (margem e
 * custo fixo); frete de venda: produto > padrão; taxa de cartão: MAIOR
 * máquina cadastrada > taxa única do tenant. Carregada uma vez por sessão. */
export function useMargens() {
    const { apiFetch } = useApi()

    const config = useState<ConfigPrecificacao>('margens-config', () => ({
        margemPadraoPct: null,
        impostoPct: 0,
        comissaoPct: 0,
        cartaoPct: 0,
        fretePct: 0,
        outrosPct: 0,
        custosFixosMensaisCentavos: null,
        vendasMensaisEstimadas: null,
        faturamentoMensalCentavos: null,
    }))
    const margensCategoria = useState<CategoriaMargem[]>('margens-categoria', () => [])
    const precificacaoProdutos = useState<ProdutoPrecificacao[]>('margens-produtos', () => [])
    const maquinas = useState<MaquinaCartao[]>('margens-maquinas', () => [])
    const custosFixosItens = useState<CustoFixo[]>('margens-custos-fixos', () => [])
    const giroProdutos = useState<GiroProduto[]>('margens-giro', () => [])
    const mixPagamento = useState<MixPagamento | null>('margens-mix-pagamento', () => null)
    // produto_id → alíquota efetiva de imposto (bps) do motor da reforma; vazio
    // quando o backend fiscal não responde (cai no imposto manual do tenant).
    const aliquotasEfetivas = useState<Record<string, number>>('margens-aliquotas-efetivas', () => ({}))
    const carregado = useState('margens-carregado', () => false)

    async function garantirCarregado() {
        if (carregado.value) return
        try {
            const [
                cfg,
                { margens },
                { produtos: overrides },
                { maquinas: maqs },
                { custos },
                { produtos: giro },
                { mix },
                { aliquotas },
            ] = await Promise.all([
                obterConfiguracoes(apiFetch),
                listarMargensCategoria(apiFetch),
                listarPrecificacaoProdutos(apiFetch),
                listarMaquinasCartao(apiFetch),
                listarCustosFixos(apiFetch),
                listarGiroProdutos(apiFetch),
                obterMixPagamento(apiFetch),
                // Degrada para o imposto manual do tenant se o fiscal falhar —
                // não derruba o painel de precificação inteiro.
                listarAliquotasEfetivas(apiFetch).catch(() => ({ aliquotas: [] })),
            ])
            config.value = {
                margemPadraoPct: bpsParaPct(cfg.margem_padrao_bps),
                impostoPct: bpsParaPct(cfg.imposto_venda_bps) ?? 0,
                comissaoPct: bpsParaPct(cfg.comissao_venda_bps) ?? 0,
                cartaoPct: bpsParaPct(cfg.taxa_cartao_bps) ?? 0,
                fretePct: bpsParaPct(cfg.frete_venda_bps) ?? 0,
                outrosPct: bpsParaPct(cfg.outras_despesas_venda_bps) ?? 0,
                custosFixosMensaisCentavos: cfg.custos_fixos_mensais_centavos,
                vendasMensaisEstimadas: cfg.vendas_mensais_estimadas,
                faturamentoMensalCentavos: cfg.faturamento_mensal_estimado_centavos,
            }
            margensCategoria.value = margens
            precificacaoProdutos.value = overrides
            maquinas.value = maqs
            custosFixosItens.value = custos
            giroProdutos.value = giro
            mixPagamento.value = mix
            aliquotasEfetivas.value = Object.fromEntries(
                aliquotas.map((a) => [a.produto_id, a.imposto_efetivo_bps]),
            )
            carregado.value = true
        } catch {
            // Painel de sugestão é opcional — sem config carregada, nada é mostrado.
        }
    }

    /** Força recarga na próxima consulta (após salvar Configurações/margens). */
    function invalidar() {
        carregado.value = false
    }

    function overrideProduto(produtoId: string | null): ProdutoPrecificacao | undefined {
        return produtoId ? precificacaoProdutos.value.find((p) => p.produto_id === produtoId) : undefined
    }

    /** Vendas suficientes para o mix de pagamento ser estatisticamente honesto. */
    const AMOSTRA_MINIMA_MIX = 20

    /** Taxa de cartão considerada: a MAIOR máquina cadastrada (conservador —
     * o lucro fecha mesmo se a venda cair na máquina mais cara), senão a taxa
     * única do tenant. Com histórico suficiente, a taxa é ponderada pela
     * participação real do cartão na receita — venda em Pix/dinheiro não paga
     * taxa, então aplicar a taxa cheia em tudo inflaria o preço. */
    function taxaCartao(): { pct: number; maquina: string | null; participacaoPct: number | null } {
        const base = maquinas.value.length
            ? (() => {
                  const pior = maquinas.value.reduce((a, b) => (b.taxa_bps > a.taxa_bps ? b : a))
                  return { pct: pior.taxa_bps / 100, maquina: pior.nome as string | null }
              })()
            : { pct: config.value.cartaoPct, maquina: null }

        const mix = mixPagamento.value
        if (mix && mix.amostra_vendas >= AMOSTRA_MINIMA_MIX && mix.participacao_cartao_bps < 10000) {
            const participacaoPct = mix.participacao_cartao_bps / 100
            return { pct: (base.pct * participacaoPct) / 100, maquina: base.maquina, participacaoPct }
        }
        return { ...base, participacaoPct: null }
    }

    /** Custo DIRETO por unidade além do custo do fornecedor (embalagem,
     * frete-compra rateado) — só quando definido explicitamente no produto ou na
     * categoria. É custo do PRODUTO (entra no custo do item), distinto da
     * cobertura de custos fixos (aluguel, salário, DAS), que é um % do preço em
     * `custoFixoPct`. Por isso um não anula o outro. */
    function custoDiretoUnitario(
        categoria: string | null,
        produtoId: string | null,
    ): { centavos: number; origem: SugestaoPreco['origemCustoDireto'] } {
        const doProduto = overrideProduto(produtoId)
        if (doProduto?.custo_fixo_unitario_centavos != null) {
            return { centavos: doProduto.custo_fixo_unitario_centavos, origem: 'produto' }
        }
        if (categoria) {
            const daCategoria = margensCategoria.value.find((m) => m.categoria === categoria)
            if (daCategoria?.custo_fixo_unitario_centavos != null) {
                return { centavos: daCategoria.custo_fixo_unitario_centavos, origem: 'categoria' }
            }
        }
        return { centavos: 0, origem: null }
    }

    /** Cobertura dos custos fixos embutida no preço, de forma TRANSPARENTE: a
     * fração do preço que recupera o custo fixo de período = custos fixos ÷
     * faturamento mensal esperado. É o "markup de cobertura" — a mesma % que o
     * painel de cobertura mostra —, aplicada a todo item para o preço fechar a
     * conta da loja no volume esperado. O dono ajusta as MARGENS por categoria
     * conforme a concorrência (cada caso); esta linha cobre só o overhead. */
    function custoFixoPct(): number {
        const { custosFixosMensaisCentavos: fixos, faturamentoMensalCentavos: faturamento } = config.value
        if (fixos != null && fixos > 0 && faturamento != null && faturamento > 0) {
            return (100 * fixos) / faturamento
        }
        return 0
    }

    /** Descontos que saem do preço para um produto específico (frete de venda
     * pode ter override por produto; cartão vem da pior máquina; custos fixos
     * entram como percentual do preço = fixos ÷ faturamento esperado). */
    function descontosDoProduto(
        produtoId: string | null,
        categoria: string | null = null,
    ): { nome: string; pct: number }[] {
        const cartao = taxaCartao()
        const rotuloCartao = [
            cartao.maquina,
            cartao.participacaoPct != null ? `${cartao.participacaoPct.toFixed(0)}% das vendas` : null,
        ]
            .filter(Boolean)
            .join(', ')
        const fretePct = bpsParaPct(overrideProduto(produtoId)?.frete_venda_bps ?? null) ?? config.value.fretePct
        // Imposto efetivo do produto pelo motor da reforma (fase vigente +
        // perfil do tenant); sem ele (produto novo/fiscal indisponível), usa o
        // imposto manual único do tenant como piso de compatibilidade.
        const impostoPct =
            (produtoId != null ? bpsParaPct(aliquotasEfetivas.value[produtoId] ?? null) : null) ??
            config.value.impostoPct
        // Custos variáveis da venda + a cobertura transparente dos custos fixos
        // (fixos ÷ faturamento). O custo fixo de período NÃO distorce por item:
        // é a mesma fração para todos; a MARGEM por categoria (cada caso) é a
        // alavanca de lucro/competitividade. O painel de cobertura valida o todo.
        return [
            { nome: 'Imposto', pct: impostoPct },
            { nome: 'Comissão', pct: config.value.comissaoPct },
            { nome: rotuloCartao ? `Cartão (${rotuloCartao})` : 'Cartão', pct: cartao.pct },
            { nome: 'Frete', pct: fretePct },
            { nome: 'Outros', pct: config.value.outrosPct },
            { nome: 'Custos fixos', pct: custoFixoPct() },
        ].filter((d) => d.pct > 0)
    }

    function margemDesejada(
        categoria: string | null,
        produtoId: string | null,
    ): { pct: number; origem: SugestaoPreco['origemMargem'] } | null {
        const doProduto = bpsParaPct(overrideProduto(produtoId)?.margem_bps ?? null)
        if (doProduto != null) return { pct: doProduto, origem: 'produto' }
        const daCategoria = categoria
            ? margensCategoria.value.find((m) => m.categoria === categoria)
            : undefined
        if (daCategoria) return { pct: daCategoria.margem_bps / 100, origem: 'categoria' }
        if (config.value.margemPadraoPct != null) {
            return { pct: config.value.margemPadraoPct, origem: 'padrao' }
        }
        return null
    }

    function giroDoProduto(produtoId: string | null): GiroProduto | undefined {
        return produtoId ? giroProdutos.value.find((g) => g.produto_id === produtoId) : undefined
    }

    /** Piso da margem após ajuste de giro: ZERO — liquidar encalhado é decisão
     * de margem de contribuição: o preço ainda cobre produto, taxas e a fatia
     * dos custos fixos, apenas abre mão do lucro para virar estoque em caixa. */
    const MARGEM_MINIMA_POS_GIRO_PCT = 0

    /** Ajuste de margem por tempo parado e volume de vendas. Regras fixas e
     * transparentes (o motivo vira um passo do cálculo na tela):
     *   parado 60–89 dias → −5 p.p. · 90–179 → −10 p.p. · 180+ → −15 p.p.
     *   vendendo bem (10+/mês) e girando → margem cheia, com o argumento. */
    function ajusteGiro(produtoId: string | null): AjusteGiro | null {
        const giro = giroDoProduto(produtoId)
        if (!giro || giro.saldo <= 0) return null

        const parado = giro.dias_sem_venda ?? giro.dias_desde_cadastro
        const nuncaVendeu = giro.dias_sem_venda == null
        const rotuloParado = nuncaVendeu
            ? `sem nenhuma venda desde o cadastro (${parado} dias)`
            : `sem vender há ${parado} dias`

        if (parado >= 180) {
            return { pontos: -15, motivo: `Produto ${rotuloParado} com ${giro.saldo} un. em estoque — reduza para girar` }
        }
        if (parado >= 90) {
            return { pontos: -10, motivo: `Produto ${rotuloParado} com ${giro.saldo} un. em estoque — reduza para girar` }
        }
        if (parado >= 60) {
            return { pontos: -5, motivo: `Produto ${rotuloParado} — um pequeno desconto ajuda a girar` }
        }
        if (giro.unidades_90d >= 30) {
            return {
                pontos: 0,
                motivo: `Vende bem (${giro.unidades_90d} un. nos últimos 90 dias) — sustenta a margem cheia`,
            }
        }
        return null
    }

    /** Preço sugerido (markup sobre o preço de venda):
     * preço = custo_direto / (1 − (deduções% + margem%) / 100). As deduções são
     * os custos variáveis da venda (imposto, comissão, cartão, frete) MAIS a
     * cobertura transparente dos custos fixos (fixos ÷ faturamento, igual para
     * todo item — não distorce por preço). A MARGEM por categoria é o lucro, e é
     * a alavanca de competitividade caso a caso; o painel de cobertura
     * (`coberturaCustosFixos`) valida se margens × volume fecham a conta. */
    function sugerirPreco(
        custoDiretoCentavos: number,
        categoria: string | null,
        produtoId: string | null = null,
    ): SugestaoPreco | SugestaoInvalida | null {
        const margem = margemDesejada(categoria, produtoId)
        if (margem == null || custoDiretoCentavos <= 0) return null

        const ajuste = ajusteGiro(produtoId)
        const margemEfetivaPct = Math.max(
            margem.pct + (ajuste?.pontos ?? 0),
            Math.min(margem.pct, MARGEM_MINIMA_POS_GIRO_PCT),
        )

        const direto = custoDiretoUnitario(categoria, produtoId)
        const custoTotalCentavos = custoDiretoCentavos + direto.centavos
        const descontos = descontosDoProduto(produtoId, categoria)
        const descontosPct = descontos.reduce((s, d) => s + d.pct, 0)
        const denominador = 1 - (descontosPct + margemEfetivaPct) / 100
        if (denominador <= 0) {
            return {
                invalido: true,
                motivo: 'Os percentuais configurados somam 100% ou mais — ajuste em Configurações antes de usar a sugestão.',
            }
        }

        const precoExato = Math.round(custoTotalCentavos / denominador)
        return {
            precoCentavos: precoExato,
            precoArredondadoCentavos: precoPsicologico(precoExato),
            custoTotalCentavos,
            custoDiretoUnitarioCentavos: direto.centavos,
            origemCustoDireto: direto.origem,
            descontos,
            margemPct: margemEfetivaPct,
            margemBasePct: margem.pct,
            origemMargem: margem.origem,
            ajusteGiro: ajuste,
        }
    }

    /** Lucro líquido por unidade num preço dado: preço − custo total −
     * descontos sobre o preço. `percentual` é a margem líquida sobre o preço. */
    function lucroLiquido(
        custoTotalCentavos: number,
        precoVendaCentavos: number,
        produtoId: string | null = null,
        categoria: string | null = null,
    ): { valorCentavos: number; percentual: number } | null {
        if (precoVendaCentavos <= 0) return null
        const descontosPct = descontosDoProduto(produtoId, categoria).reduce((s, d) => s + d.pct, 0)
        const descontosCentavos = Math.round((precoVendaCentavos * descontosPct) / 100)
        const valorCentavos = precoVendaCentavos - custoTotalCentavos - descontosCentavos
        return { valorCentavos, percentual: (valorCentavos / precoVendaCentavos) * 100 }
    }

    /** Quantas unidades são necessárias por mês só para cobrir os custos
     * fixos. Margem de contribuição unitária = preço − custo − taxas variáveis
     * sobre o preço (imposto, comissão, cartão efetivo, frete, outros — SEM os
     * custos fixos, que são o numerador do break-even), ponderada pelo mix
     * real de vendas (unidades 90d); sem histórico, média simples do catálogo. */
    function pontoDeEquilibrio(
        produtos: Produto[],
    ): { unidades: number; receitaCentavos: number; margemContribuicaoMediaCentavos: number } | null {
        const fixos = config.value.custosFixosMensaisCentavos
        if (fixos == null || fixos <= 0) return null

        // Taxas variáveis genéricas (sem override por produto e sem fixos).
        const cartao = taxaCartao()
        const taxasPct =
            config.value.impostoPct +
            config.value.comissaoPct +
            cartao.pct +
            config.value.fretePct +
            config.value.outrosPct

        const contribuicao = (p: Produto) =>
            p.preco_venda - p.preco_custo - (p.preco_venda * taxasPct) / 100

        const ativos = produtos.filter((p) => p.ativo && contribuicao(p) > 0)
        if (!ativos.length) return null

        // Peso = unidades vendidas em 90d (mix real); fallback peso 1 para todos.
        const pesoDe = (p: Produto) => giroDoProduto(p.produto_id)?.unidades_90d ?? 0
        const temHistorico = ativos.some((p) => pesoDe(p) > 0)
        const somaPesos = ativos.reduce((s, p) => s + (temHistorico ? pesoDe(p) : 1), 0)
        if (somaPesos <= 0) return null

        const margemMedia =
            ativos.reduce((s, p) => s + contribuicao(p) * (temHistorico ? pesoDe(p) : 1), 0) / somaPesos
        const precoMedio =
            ativos.reduce((s, p) => s + p.preco_venda * (temHistorico ? pesoDe(p) : 1), 0) / somaPesos
        if (margemMedia <= 0) return null

        const unidades = Math.ceil(fixos / margemMedia)
        return {
            unidades,
            receitaCentavos: Math.round(unidades * precoMedio),
            margemContribuicaoMediaCentavos: Math.round(margemMedia),
        }
    }

    /** Cobertura dos custos fixos no NÍVEL DA LOJA. Cada preço já embute uma
     * fração de cobertura (`custoFixoPct` = fixos ÷ faturamento); este painel
     * VALIDA se, no volume esperado, a margem de contribuição agregada realmente
     * cobre os custos fixos. Devolve o ponto de equilíbrio, o gap mensal e a
     * "margem de cobertura" de referência (fixos ÷ faturamento) — o mesmo % que
     * entra no preço, exposto para o dono calibrar as margens por categoria
     * conforme concorrência/elasticidade (cada caso é um caso). */
    function coberturaCustosFixos(produtos: Produto[]): {
        fixosCentavos: number
        equilibrio: ReturnType<typeof pontoDeEquilibrio>
        vendasEsperadas: number | null
        contribuicaoEsperadaCentavos: number | null
        cobre: boolean | null
        gapMensalCentavos: number
        markupCoberturaPct: number | null
    } | null {
        const fixos = config.value.custosFixosMensaisCentavos
        if (fixos == null || fixos <= 0) return null

        const equilibrio = pontoDeEquilibrio(produtos)
        const faturamento = config.value.faturamentoMensalCentavos
        const markupCoberturaPct =
            faturamento != null && faturamento > 0 ? (100 * fixos) / faturamento : null

        const vendasEsperadas = config.value.vendasMensaisEstimadas
        let cobre: boolean | null = null
        let gapMensalCentavos = 0
        let contribuicaoEsperadaCentavos: number | null = null
        if (equilibrio && vendasEsperadas != null && vendasEsperadas > 0) {
            contribuicaoEsperadaCentavos =
                equilibrio.margemContribuicaoMediaCentavos * vendasEsperadas
            cobre = contribuicaoEsperadaCentavos >= fixos
            gapMensalCentavos = Math.max(0, fixos - contribuicaoEsperadaCentavos)
        }
        return {
            fixosCentavos: fixos,
            equilibrio,
            vendasEsperadas,
            contribuicaoEsperadaCentavos,
            cobre,
            gapMensalCentavos,
            markupCoberturaPct,
        }
    }

    return {
        config,
        margensCategoria,
        precificacaoProdutos,
        maquinas,
        custosFixosItens,
        giroDoProduto,
        garantirCarregado,
        invalidar,
        sugerirPreco,
        lucroLiquido,
        pontoDeEquilibrio,
        coberturaCustosFixos,
    }
}
