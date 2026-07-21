import type { Produto } from '~/models/catalogo'
import {
    alternarAtivoProduto,
    atualizarPrecos,
    atualizarProduto,
    criarProduto,
    definirPrecificacaoProduto,
    listarPrecificacaoProdutos,
    listarProdutos,
} from '~/models/catalogo'

/** ViewModel da página de Catálogo: concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useCatalogoViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { toCentavos } = useFormat()
    const { notifySuccess, notifyError } = useNotify()
    const { buscarAproximado } = useFuzzySearch()

    const produtos = ref<Produto[]>([])
    const loading = ref(false)
    const filtro = ref('')

    const { printCatalogReport } = useThermalPrint()
    const { tenantSlug } = useAuth()

    /** Catálogo impresso (produtos ativos, respeitando o filtro da tela). */
    function imprimirCatalogo() {
        printCatalogReport(
            tenantSlug.value || 'Finledger',
            produtosFiltrados.value
                .filter((p) => p.ativo)
                .map((p) => ({
                    sku: p.sku,
                    descricao: p.descricao,
                    categoria: p.categoria,
                    precoVendaCents: p.preco_venda,
                })),
        )
    }

    const produtosFiltrados = computed(() =>
        buscarAproximado(produtos.value, filtro.value, (p) => `${p.descricao} ${p.sku} ${p.categoria} ${p.marca ?? ''}`),
    )

    async function carregar() {
        loading.value = true
        try {
            const { produtos: lista } = await listarProdutos(apiFetch)
            produtos.value = lista
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    // --- Cadastro / edição ---
    const dialogVisible = ref(false)
    const editando = ref<Produto | null>(null)
    const salvando = ref(false)
    const form = reactive({
        sku: '',
        descricao: '',
        ncm: '',
        unidade: 'UN',
        categoria: '',
        marca: '',
        preco_custo: 0,
        preco_venda: 0,
        controla_estoque: true,
    })

    function abrirNovo() {
        editando.value = null
        Object.assign(form, {
            sku: '',
            descricao: '',
            ncm: '',
            unidade: 'UN',
            categoria: '',
            marca: '',
            preco_custo: 0,
            preco_venda: 0,
            controla_estoque: true,
        })
        dialogVisible.value = true
    }

    /** Ajustes de precificação deste produto (todos opcionais; vazio = usa o
     * da categoria/padrão da loja). Só editáveis em produto existente. */
    const ajustesProduto = reactive({
        margemPct: null as number | null,
        custoFixoUnitario: null as number | null, // em reais na tela
        freteVendaPct: null as number | null,
    })
    const ajustesOriginais = ref('')
    const { invalidar: invalidarMargens } = useMargens()

    function abrirEdicao(p: Produto) {
        editando.value = p
        Object.assign(form, {
            sku: p.sku,
            descricao: p.descricao,
            ncm: p.ncm,
            unidade: p.unidade,
            categoria: p.categoria,
            marca: p.marca ?? '',
            preco_custo: p.preco_custo / 100,
            preco_venda: p.preco_venda / 100,
            controla_estoque: p.controla_estoque,
        })
        Object.assign(ajustesProduto, { margemPct: null, custoFixoUnitario: null, freteVendaPct: null })
        ajustesOriginais.value = JSON.stringify(ajustesProduto)
        void listarPrecificacaoProdutos(apiFetch)
            .then(({ produtos: overrides }) => {
                const o = overrides.find((x) => x.produto_id === p.produto_id)
                Object.assign(ajustesProduto, {
                    margemPct: o?.margem_bps == null ? null : o.margem_bps / 100,
                    custoFixoUnitario:
                        o?.custo_fixo_unitario_centavos == null ? null : o.custo_fixo_unitario_centavos / 100,
                    freteVendaPct: o?.frete_venda_bps == null ? null : o.frete_venda_bps / 100,
                })
                ajustesOriginais.value = JSON.stringify(ajustesProduto)
            })
            .catch(() => {})
        dialogVisible.value = true
    }

    async function salvarAjustesProduto(produtoId: string) {
        if (JSON.stringify(ajustesProduto) === ajustesOriginais.value) return
        await definirPrecificacaoProduto(apiFetch, produtoId, {
            margem_bps: ajustesProduto.margemPct == null ? null : Math.round(ajustesProduto.margemPct * 100),
            custo_fixo_unitario_centavos:
                ajustesProduto.custoFixoUnitario == null ? null : toCentavos(ajustesProduto.custoFixoUnitario),
            frete_venda_bps:
                ajustesProduto.freteVendaPct == null ? null : Math.round(ajustesProduto.freteVendaPct * 100),
        })
        invalidarMargens()
    }

    async function salvar() {
        salvando.value = true
        try {
            if (editando.value) {
                await atualizarProduto(apiFetch, editando.value.produto_id, {
                    sku: form.sku,
                    descricao: form.descricao,
                    ncm: form.ncm,
                    unidade: form.unidade,
                    categoria: form.categoria,
                    marca: form.marca.trim() || null,
                    controla_estoque: form.controla_estoque,
                })
                await atualizarPrecos(apiFetch, editando.value.produto_id, {
                    preco_custo_centavos: toCentavos(form.preco_custo),
                    preco_venda_centavos: toCentavos(form.preco_venda),
                })
                await salvarAjustesProduto(editando.value.produto_id)
            } else {
                await criarProduto(apiFetch, {
                    sku: form.sku,
                    descricao: form.descricao,
                    ncm: form.ncm,
                    unidade: form.unidade,
                    categoria: form.categoria,
                    marca: form.marca.trim() || null,
                    controla_estoque: form.controla_estoque,
                    preco_custo_centavos: toCentavos(form.preco_custo),
                    preco_venda_centavos: toCentavos(form.preco_venda),
                })
            }
            notifySuccess('Salvo', 'Produto salvo.')
            dialogVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvando.value = false
        }
    }

    async function alternarAtivo(p: Produto) {
        try {
            await alternarAtivoProduto(apiFetch, p.produto_id, p.ativo)
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    return reactive({
        produtos,
        loading,
        filtro,
        produtosFiltrados,
        carregar,
        imprimirCatalogo,
        dialogVisible,
        editando,
        salvando,
        form,
        ajustesProduto,
        abrirNovo,
        abrirEdicao,
        salvar,
        alternarAtivo,
    })
}
