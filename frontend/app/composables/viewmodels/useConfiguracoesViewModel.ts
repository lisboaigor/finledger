import {
    atualizarConfiguracoes,
    definirCustoFixo,
    listarCustosFixos,
    obterConfiguracoes,
    removerCustoFixo,
} from '~/models/configuracoes'
import type { CustoFixo } from '~/models/configuracoes'
import type { CategoriaMargem, MaquinaCartao, Produto } from '~/models/catalogo'
import {
    definirMargemCategoria,
    definirMaquinaCartao,
    listarCategorias,
    listarMargensCategoria,
    listarMaquinasCartao,
    listarProdutos,
    removerMargemCategoria,
    removerMaquinaCartao,
} from '~/models/catalogo'

/** ViewModel da página de Configurações: feature flag de estoque em orçamentos,
 * dados da empresa exibidos nas impressões e precificação assistida (margens,
 * descontos do preço, custos fixos e ponto de equilíbrio). */
export function useConfiguracoesViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { isAdmin } = useAuth()
    const { notifySuccess, notifyError } = useNotify()
    const { invalidar: invalidarMargens, pontoDeEquilibrio, garantirCarregado } = useMargens()

    const loading = ref(true)
    const salvando = ref(false)
    const permiteOrcamentoSemEstoque = ref(true)
    const arquivamentoDias = ref<number | null>(null)
    const cnpj = ref('')
    const telefone = ref('')
    const endereco = ref('')
    const chavePix = ref('')
    const informacoesAdicionais = ref('')

    // --- Precificação (percentuais em % na tela; bps só na API) ---
    const margemPadraoPct = ref<number | null>(null)
    const impostoPct = ref<number | null>(null)
    const comissaoPct = ref<number | null>(null)
    const cartaoPct = ref<number | null>(null)
    const fretePct = ref<number | null>(null)
    const outrosPct = ref<number | null>(null)
    const custosFixosMensais = ref<number | null>(null) // em reais na tela
    const vendasMensaisEstimadas = ref<number | null>(null)
    const faturamentoMensal = ref<number | null>(null) // em reais na tela
    const metaFaturamento = ref<number | null>(null) // em reais na tela

    const pctParaBps = (pct: number | null) => (pct == null ? null : Math.round(pct * 100))
    const bpsParaPct = (bps: number | null) => (bps == null ? null : bps / 100)

    /** Percentual do preço que cobre custos fixos (rateio proporcional). */
    const custoFixoPct = computed(() => {
        if (!custosFixosMensais.value || !faturamentoMensal.value) return 0
        return (100 * custosFixosMensais.value) / faturamentoMensal.value
    })

    /** Soma dos percentuais que saem do preço + margem — aviso quando ≥ 100%. */
    const percentuaisInvalidos = computed(() => {
        const soma =
            (margemPadraoPct.value ?? 0) +
            (impostoPct.value ?? 0) +
            (comissaoPct.value ?? 0) +
            (cartaoPct.value ?? 0) +
            (fretePct.value ?? 0) +
            (outrosPct.value ?? 0) +
            custoFixoPct.value
        return soma >= 100
    })

    async function carregar() {
        loading.value = true
        try {
            const cfg = await obterConfiguracoes(apiFetch)
            permiteOrcamentoSemEstoque.value = cfg.permite_orcamento_sem_estoque
            arquivamentoDias.value = cfg.arquivamento_dias
            cnpj.value = cfg.cnpj ?? ''
            telefone.value = cfg.telefone ?? ''
            endereco.value = cfg.endereco ?? ''
            chavePix.value = cfg.chave_pix ?? ''
            informacoesAdicionais.value = cfg.informacoes_adicionais ?? ''
            margemPadraoPct.value = bpsParaPct(cfg.margem_padrao_bps)
            impostoPct.value = bpsParaPct(cfg.imposto_venda_bps)
            comissaoPct.value = bpsParaPct(cfg.comissao_venda_bps)
            cartaoPct.value = bpsParaPct(cfg.taxa_cartao_bps)
            fretePct.value = bpsParaPct(cfg.frete_venda_bps)
            outrosPct.value = bpsParaPct(cfg.outras_despesas_venda_bps)
            custosFixosMensais.value =
                cfg.custos_fixos_mensais_centavos == null ? null : cfg.custos_fixos_mensais_centavos / 100
            vendasMensaisEstimadas.value = cfg.vendas_mensais_estimadas
            faturamentoMensal.value =
                cfg.faturamento_mensal_estimado_centavos == null
                    ? null
                    : cfg.faturamento_mensal_estimado_centavos / 100
            metaFaturamento.value =
                cfg.meta_faturamento_mensal_centavos == null
                    ? null
                    : cfg.meta_faturamento_mensal_centavos / 100
            await Promise.all([carregarMargens(), carregarCustosFixos(), carregarBreakEven()])
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    async function salvar() {
        salvando.value = true
        try {
            await atualizarConfiguracoes(apiFetch, {
                permite_orcamento_sem_estoque: permiteOrcamentoSemEstoque.value,
                arquivamento_dias: arquivamentoDias.value,
                cnpj: cnpj.value.trim() || null,
                telefone: telefone.value.trim() || null,
                endereco: endereco.value.trim() || null,
                chave_pix: chavePix.value.trim() || null,
                informacoes_adicionais: informacoesAdicionais.value.trim() || null,
                margem_padrao_bps: pctParaBps(margemPadraoPct.value),
                imposto_venda_bps: pctParaBps(impostoPct.value),
                comissao_venda_bps: pctParaBps(comissaoPct.value),
                taxa_cartao_bps: pctParaBps(cartaoPct.value),
                frete_venda_bps: pctParaBps(fretePct.value),
                outras_despesas_venda_bps: pctParaBps(outrosPct.value),
                custos_fixos_mensais_centavos:
                    custosFixosMensais.value == null ? null : Math.round(custosFixosMensais.value * 100),
                vendas_mensais_estimadas: vendasMensaisEstimadas.value,
                faturamento_mensal_estimado_centavos:
                    faturamentoMensal.value == null ? null : Math.round(faturamentoMensal.value * 100),
                meta_faturamento_mensal_centavos:
                    metaFaturamento.value == null ? null : Math.round(metaFaturamento.value * 100),
            })
            invalidarMargens()
            notifySuccess('Salvo', 'Configurações atualizadas.')
            await carregarBreakEven()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvando.value = false
        }
    }

    // --- Margens por categoria ---
    const margens = ref<CategoriaMargem[]>([])
    const categorias = ref<string[]>([])
    const novaMargem = reactive({
        categoria: '' as string,
        margemPct: null as number | null,
        custoFixoUnitario: null as number | null, // em reais na tela
    })
    const salvandoMargem = ref(false)

    async function carregarMargens() {
        const [{ margens: m }, { categorias: c }, { maquinas: maq }] = await Promise.all([
            listarMargensCategoria(apiFetch),
            listarCategorias(apiFetch),
            listarMaquinasCartao(apiFetch),
        ])
        margens.value = m
        categorias.value = c
        maquinas.value = maq
    }

    // --- Máquinas de cartão ---
    const maquinas = ref<MaquinaCartao[]>([])
    const novaMaquina = reactive({ nome: '', taxaPct: null as number | null })
    const salvandoMaquina = ref(false)

    async function salvarMaquina() {
        if (!novaMaquina.nome.trim() || novaMaquina.taxaPct == null) return
        salvandoMaquina.value = true
        try {
            await definirMaquinaCartao(apiFetch, {
                nome: novaMaquina.nome.trim(),
                taxa_bps: Math.round(novaMaquina.taxaPct * 100),
            })
            Object.assign(novaMaquina, { nome: '', taxaPct: null })
            invalidarMargens()
            notifySuccess('Salvo', 'Máquina de cartão atualizada.')
            await carregarMargens()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoMaquina.value = false
        }
    }

    function editarMaquina(m: MaquinaCartao) {
        Object.assign(novaMaquina, { nome: m.nome, taxaPct: m.taxa_bps / 100 })
    }

    async function removerMaquina(nome: string) {
        try {
            await removerMaquinaCartao(apiFetch, nome)
            invalidarMargens()
            notifySuccess('Removido', `Máquina ${nome} removida.`)
            await carregarMargens()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    async function salvarMargem() {
        if (!novaMargem.categoria.trim() || novaMargem.margemPct == null) return
        salvandoMargem.value = true
        try {
            await definirMargemCategoria(apiFetch, {
                categoria: novaMargem.categoria.trim(),
                margem_bps: Math.round(novaMargem.margemPct * 100),
                custo_fixo_unitario_centavos:
                    novaMargem.custoFixoUnitario == null ? null : Math.round(novaMargem.custoFixoUnitario * 100),
            })
            Object.assign(novaMargem, { categoria: '', margemPct: null, custoFixoUnitario: null })
            invalidarMargens()
            notifySuccess('Salvo', 'Margem da categoria atualizada.')
            await carregarMargens()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoMargem.value = false
        }
    }

    function editarMargem(m: CategoriaMargem) {
        Object.assign(novaMargem, {
            categoria: m.categoria,
            margemPct: m.margem_bps / 100,
            custoFixoUnitario:
                m.custo_fixo_unitario_centavos == null ? null : m.custo_fixo_unitario_centavos / 100,
        })
    }

    async function removerMargem(categoria: string) {
        try {
            await removerMargemCategoria(apiFetch, categoria)
            invalidarMargens()
            notifySuccess('Removido', `Categoria ${categoria} voltou a usar a margem padrão.`)
            await carregarMargens()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Custos fixos discriminados ---
    // Com itens cadastrados, o total mensal vira a soma deles (mantida pelo
    // backend) e o campo único fica somente leitura na tela.
    const custosFixos = ref<CustoFixo[]>([])
    const novoCusto = reactive({ nome: '', valor: null as number | null }) // valor em reais na tela
    const salvandoCusto = ref(false)

    async function carregarCustosFixos() {
        const { custos } = await listarCustosFixos(apiFetch)
        custosFixos.value = custos
    }

    /** Recarrega o total (o backend sincroniza a soma em tenants). */
    async function atualizarTotalCustosFixos() {
        const cfg = await obterConfiguracoes(apiFetch)
        custosFixosMensais.value =
            cfg.custos_fixos_mensais_centavos == null ? null : cfg.custos_fixos_mensais_centavos / 100
    }

    async function salvarCusto() {
        if (!novoCusto.nome.trim() || novoCusto.valor == null) return
        salvandoCusto.value = true
        try {
            await definirCustoFixo(apiFetch, {
                nome: novoCusto.nome.trim(),
                valor_centavos: Math.round(novoCusto.valor * 100),
            })
            Object.assign(novoCusto, { nome: '', valor: null })
            invalidarMargens()
            notifySuccess('Salvo', 'Custo fixo atualizado.')
            await Promise.all([carregarCustosFixos(), atualizarTotalCustosFixos()])
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoCusto.value = false
        }
    }

    function editarCusto(c: CustoFixo) {
        Object.assign(novoCusto, { nome: c.nome, valor: c.valor_centavos / 100 })
    }

    async function removerCusto(nome: string) {
        try {
            await removerCustoFixo(apiFetch, nome)
            invalidarMargens()
            notifySuccess('Removido', `Custo "${nome}" removido.`)
            await Promise.all([carregarCustosFixos(), atualizarTotalCustosFixos()])
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Ponto de equilíbrio ---
    const produtos = ref<Produto[]>([])

    async function carregarBreakEven() {
        try {
            const { produtos: p } = await listarProdutos(apiFetch)
            produtos.value = p
            await garantirCarregado()
        } catch {
            // cards de break-even são opcionais
        }
    }

    const breakEven = computed(() => pontoDeEquilibrio(produtos.value))

    return reactive({
        loading,
        salvando,
        isAdmin,
        permiteOrcamentoSemEstoque,
        arquivamentoDias,
        cnpj,
        telefone,
        endereco,
        chavePix,
        informacoesAdicionais,
        // precificação
        margemPadraoPct,
        impostoPct,
        comissaoPct,
        cartaoPct,
        fretePct,
        outrosPct,
        custosFixosMensais,
        vendasMensaisEstimadas,
        faturamentoMensal,
        metaFaturamento,
        custoFixoPct,
        percentuaisInvalidos,
        // margens por categoria
        margens,
        categorias,
        novaMargem,
        salvandoMargem,
        salvarMargem,
        editarMargem,
        removerMargem,
        // máquinas de cartão
        maquinas,
        novaMaquina,
        salvandoMaquina,
        salvarMaquina,
        editarMaquina,
        removerMaquina,
        // custos fixos discriminados
        custosFixos,
        novoCusto,
        salvandoCusto,
        salvarCusto,
        editarCusto,
        removerCusto,
        // break-even
        breakEven,
        carregar,
        salvar,
    })
}
