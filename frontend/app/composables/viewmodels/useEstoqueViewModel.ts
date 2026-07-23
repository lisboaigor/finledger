import type { Opcao } from '~/models/shared'
import type { Produto } from '~/models/catalogo'
import { atualizarPrecos, listarFretesFornecedor, listarProdutos } from '~/models/catalogo'
import type { FornecedorFrete } from '~/models/catalogo'
import type { Fornecedor } from '~/models/fornecedores'
import { listarFornecedores } from '~/models/fornecedores'
import type { Saldo } from '~/models/estoque'
import { ajustarSaldo, definirMinimo, listarSaldos, registrarEntrada } from '~/models/estoque'

/** ViewModel da página de Estoque: concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useEstoqueViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { toCentavos } = useFormat()
    const { printInventoryReport } = useThermalPrint()
    const { tenantSlug } = useAuth()
    const { notifySuccess, notifyError } = useNotify()

    const saldos = ref<Saldo[]>([])
    const produtos = ref<Produto[]>([])
    const loading = ref(false)
    const busca = ref('')

    const produtoPorId = computed(() => {
        const map = new Map<string, Produto>()
        produtos.value.forEach((p) => map.set(p.produto_id, p))
        return map
    })

    const linhas = computed(() =>
        saldos.value.map((s) => {
            const p = produtoPorId.value.get(s.produto_id)
            return {
                ...s,
                sku: p?.sku ?? '—',
                descricao: p?.descricao ?? s.produto_id,
                categoria: p?.categoria,
                marca: p?.marca,
                precoCustoCents: p?.preco_custo,
                precoVendaCents: p?.preco_venda,
            }
        }),
    )

    const { buscarAproximado } = useFuzzySearch()

    const linhasFiltradas = computed(() =>
        buscarAproximado(linhas.value, busca.value, (l) => `${l.sku} ${l.descricao} ${l.categoria ?? ''} ${l.marca ?? ''}`),
    )

    async function carregar() {
        loading.value = true
        try {
            const [{ saldos: s }, { produtos: p }] = await Promise.all([
                listarSaldos(apiFetch),
                listarProdutos(apiFetch),
            ])
            saldos.value = s
            produtos.value = p
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    // --- Inventory valuation at average cost ---
    const totalUnidades = computed(() => saldos.value.reduce((s, x) => s + x.quantidade, 0))
    const valorCustoCents = computed(() =>
        saldos.value.reduce((s, x) => s + x.quantidade * x.custo_medio, 0),
    )
    const itensComSaldo = computed(() => saldos.value.filter((s) => s.quantidade > 0).length)

    function imprimirInventario() {
        printInventoryReport(
            tenantSlug.value ?? 'Finledger',
            linhas.value
                .filter((l) => l.quantidade > 0)
                .map((l) => ({
                    sku: l.sku,
                    descricao: l.descricao,
                    categoria: l.categoria,
                    quantidade: l.quantidade,
                    custoMedioCents: l.custo_medio,
                    precoCustoCents: l.precoCustoCents,
                    precoVendaCents: l.precoVendaCents,
                })),
        )
    }

    const opcoesProduto = computed<Opcao[]>(() =>
        produtos.value
            .filter((p) => p.ativo)
            .map((p) => ({ label: `${p.sku} — ${p.descricao}`, value: p.produto_id })),
    )

    // --- Entrada de estoque ---
    const entradaVisible = ref(false)
    const salvandoEntrada = ref(false)
    const entrada = reactive({
        produto_id: null as string | null,
        fornecedor_id: null as string | null,
        quantidade: 1,
        custo_unitario: 0,
        /** Frete total da remessa (R$) — rateado pelas unidades e somado ao
         * custo unitário que entra no estoque (custo "posto na loja"). */
        frete_remessa: 0,
        motivo: '',
        nota_fiscal: '',
    })

    // --- Fornecedores (para pré-preencher o frete típico) ---
    const fornecedores = ref<Fornecedor[]>([])
    const fretesFornecedor = ref<FornecedorFrete[]>([])

    const opcoesFornecedor = computed<Opcao[]>(() =>
        fornecedores.value
            .filter((f) => f.ativo)
            .map((f) => ({ label: f.razao_social, value: f.fornecedor_id })),
    )

    async function carregarFornecedores() {
        try {
            const [{ fornecedores: f }, { fretes }] = await Promise.all([
                listarFornecedores(apiFetch),
                listarFretesFornecedor(apiFetch),
            ])
            fornecedores.value = f
            fretesFornecedor.value = fretes
        } catch {
            // campo de fornecedor na entrada é opcional
        }
    }

    /** Custo unitário efetivo ("posto na loja"): custo + rateio do frete da
     * remessa. É o que entra no estoque e o que a sugestão de preço usa. */
    const custoEfetivoUnitarioCentavos = computed(() => {
        const custo = toCentavos(entrada.custo_unitario)
        if (entrada.frete_remessa > 0 && entrada.quantidade > 0) {
            return custo + Math.round(toCentavos(entrada.frete_remessa) / entrada.quantidade)
        }
        return custo
    })

    // Pré-preenche o frete da remessa com o % típico do fornecedor escolhido
    // (frete = % sobre o valor da mercadoria). Só quando o campo está zerado,
    // pra não sobrescrever um valor digitado.
    watch(
        () => [entrada.fornecedor_id, entrada.custo_unitario, entrada.quantidade] as const,
        () => {
            if (!entrada.fornecedor_id || entrada.frete_remessa > 0) return
            const frete = fretesFornecedor.value.find((f) => f.fornecedor_id === entrada.fornecedor_id)
            if (!frete) return
            const valorMercadoria = entrada.custo_unitario * entrada.quantidade
            entrada.frete_remessa = Math.round(valorMercadoria * frete.frete_tipico_bps) / 10000
        },
    )

    // Precificação assistida na entrada: o gestor pode, opcionalmente, já
    // atualizar o preço de custo/venda do produto com base no custo que
    // acabou de digitar. Nada acontece sem marcar o checkbox.
    const { isAdmin } = useAuth()
    const { sugerirPreco, garantirCarregado: garantirMargensCarregadas } = useMargens()
    const atualizarPrecoVenda = ref(false)
    const novoPrecoVenda = ref<number | null>(null) // em reais na tela

    const produtoDaEntrada = computed(() =>
        entrada.produto_id ? (produtoPorId.value.get(entrada.produto_id) ?? null) : null,
    )

    const sugestaoEntrada = computed(() => {
        const p = produtoDaEntrada.value
        if (!p || entrada.custo_unitario <= 0) return null
        const s = sugerirPreco(custoEfetivoUnitarioCentavos.value, p.categoria, p.produto_id)
        return s && !('invalido' in s) ? s : null
    })

    // Pré-preenche o campo com a sugestão quando o checkbox liga (o campo
    // continua editável — o gestor decide o valor final).
    watch(atualizarPrecoVenda, (ligado) => {
        if (ligado) {
            novoPrecoVenda.value = sugestaoEntrada.value
                ? sugestaoEntrada.value.precoCentavos / 100
                : (produtoDaEntrada.value?.preco_venda ?? 0) / 100 || null
        }
    })

    function abrirEntrada() {
        Object.assign(entrada, {
            produto_id: null,
            fornecedor_id: null,
            quantidade: 1,
            custo_unitario: 0,
            frete_remessa: 0,
            motivo: '',
            nota_fiscal: '',
        })
        atualizarPrecoVenda.value = false
        novoPrecoVenda.value = null
        void garantirMargensCarregadas()
        void carregarFornecedores()
        entradaVisible.value = true
    }

    async function registrarEntradaAtual() {
        if (!entrada.produto_id) return
        salvandoEntrada.value = true
        try {
            await registrarEntrada(apiFetch, {
                produto_id: entrada.produto_id,
                quantidade: entrada.quantidade,
                // custo "posto na loja": inclui o rateio do frete da remessa
                custo_unitario_centavos: custoEfetivoUnitarioCentavos.value,
                motivo: entrada.motivo,
                nota_fiscal: entrada.nota_fiscal.trim() || null,
            })
            if (atualizarPrecoVenda.value && novoPrecoVenda.value != null && novoPrecoVenda.value > 0) {
                await atualizarPrecos(apiFetch, entrada.produto_id, {
                    preco_custo_centavos: custoEfetivoUnitarioCentavos.value,
                    preco_venda_centavos: toCentavos(novoPrecoVenda.value),
                })
                notifySuccess('Registrado', 'Entrada registrada e preço do produto atualizado.')
            } else {
                notifySuccess('Registrado', 'Entrada registrada.')
            }
            entradaVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoEntrada.value = false
        }
    }

    // --- Ajuste de saldo ---
    const ajusteVisible = ref(false)
    const salvandoAjuste = ref(false)
    const ajuste = reactive({
        produto_id: '',
        quantidade_atual: 0,
        quantidade_nova: 0,
        // Em unidades de moeda (convertido para centavos no envio).
        custo_unitario: 0,
        justificativa: '',
    })

    // Ajuste para cima exige custo das unidades acrescentadas (senão o custo
    // médio ficaria diluído com unidades "de graça").
    const ajusteAumenta = computed(() => ajuste.quantidade_nova > ajuste.quantidade_atual)

    function abrirAjuste(linha: { produto_id: string; quantidade: number; custo_medio: number }) {
        Object.assign(ajuste, {
            produto_id: linha.produto_id,
            quantidade_atual: linha.quantidade,
            quantidade_nova: linha.quantidade,
            custo_unitario: linha.custo_medio / 100,
            justificativa: '',
        })
        ajusteVisible.value = true
    }

    async function registrarAjuste() {
        salvandoAjuste.value = true
        try {
            await ajustarSaldo(apiFetch, ajuste.produto_id, {
                quantidade_nova: ajuste.quantidade_nova,
                custo_unitario_centavos: ajusteAumenta.value ? toCentavos(ajuste.custo_unitario) : null,
                justificativa: ajuste.justificativa,
            })
            notifySuccess('Ajustado', 'Saldo ajustado.')
            ajusteVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoAjuste.value = false
        }
    }

    // --- Estoque mínimo ---
    const minimoVisible = ref(false)
    const salvandoMinimo = ref(false)
    const minimo = reactive({ produto_id: '', estoque_minimo: 0 })

    function abrirMinimo(linha: { produto_id: string; estoque_minimo: number }) {
        Object.assign(minimo, {
            produto_id: linha.produto_id,
            estoque_minimo: linha.estoque_minimo,
        })
        minimoVisible.value = true
    }

    async function salvarMinimo() {
        salvandoMinimo.value = true
        try {
            await definirMinimo(apiFetch, minimo.produto_id, minimo.estoque_minimo)
            notifySuccess('Salvo', 'Estoque mínimo atualizado.')
            minimoVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoMinimo.value = false
        }
    }

    return reactive({
        totalUnidades,
        valorCustoCents,
        itensComSaldo,
        imprimirInventario,
        saldos,
        produtos,
        loading,
        busca,
        linhas,
        linhasFiltradas,
        carregar,
        opcoesProduto,
        entradaVisible,
        salvandoEntrada,
        entrada,
        isAdmin,
        produtoDaEntrada,
        opcoesFornecedor,
        custoEfetivoUnitarioCentavos,
        atualizarPrecoVenda,
        novoPrecoVenda,
        abrirEntrada,
        registrarEntrada: registrarEntradaAtual,
        ajusteVisible,
        salvandoAjuste,
        ajuste,
        ajusteAumenta,
        abrirAjuste,
        registrarAjuste,
        minimoVisible,
        salvandoMinimo,
        minimo,
        abrirMinimo,
        salvarMinimo,
    })
}
