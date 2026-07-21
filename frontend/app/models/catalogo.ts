import type { ApiFetch } from './shared'

export interface Produto {
    produto_id: string
    sku: string
    descricao: string
    ncm: string
    unidade: string
    preco_custo: number
    preco_venda: number
    categoria: string
    marca: string | null
    ativo: boolean
    /** FALSE para serviços/mão de obra — ficam fora da checagem de estoque
     * em vendas e orçamentos. */
    controla_estoque: boolean
    /** Classe tributária (cClassTrib) da reforma; null = tributação integral. */
    c_class_trib: string | null
}

export interface ProdutoDados {
    sku: string
    descricao: string
    ncm: string
    unidade: string
    categoria: string
    marca: string | null
    controla_estoque: boolean
    classe_trib: string | null
}

export interface ProdutoPrecos {
    preco_custo_centavos: number
    preco_venda_centavos: number
}

export function listarProdutos(apiFetch: ApiFetch) {
    return apiFetch<{ produtos: Produto[] }>('/catalogo/produtos')
}

export function criarProduto(apiFetch: ApiFetch, dados: ProdutoDados & ProdutoPrecos) {
    return apiFetch('/catalogo/produtos', { method: 'POST', body: dados })
}

export function atualizarProduto(apiFetch: ApiFetch, produtoId: string, dados: ProdutoDados) {
    return apiFetch(`/catalogo/produtos/${produtoId}`, { method: 'PUT', body: dados })
}

export function atualizarPrecos(apiFetch: ApiFetch, produtoId: string, precos: ProdutoPrecos) {
    return apiFetch(`/catalogo/produtos/${produtoId}/precos`, { method: 'PUT', body: precos })
}

export function alternarAtivoProduto(apiFetch: ApiFetch, produtoId: string, ativo: boolean) {
    return apiFetch(`/catalogo/produtos/${produtoId}/${ativo ? 'desativar' : 'reativar'}`, { method: 'POST' })
}

// ── Precificação assistida ───────────────────────────────────────────────────

export interface CategoriaMargem {
    categoria: string
    /** Basis points: 4000 = 40%. */
    margem_bps: number
    custo_fixo_unitario_centavos: number | null
}

/** Overrides de precificação por produto — prevalecem sobre categoria/loja. */
export interface ProdutoPrecificacao {
    produto_id: string
    margem_bps: number | null
    custo_fixo_unitario_centavos: number | null
    frete_venda_bps: number | null
}

export interface MaquinaCartao {
    nome: string
    taxa_bps: number
}

/** Giro por produto: ritmo de venda e tempo parado — insumo do ajuste de
 * margem por encalhe/volume na sugestão de preço. */
export interface GiroProduto {
    produto_id: string
    unidades_90d: number
    /** Dias desde a última venda confirmada; null = nunca vendeu. */
    dias_sem_venda: number | null
    dias_desde_cadastro: number
    saldo: number
    /** Custo médio real do estoque (0 = sem estoque/registro). */
    custo_medio_centavos: number
}

/** Participação do cartão na receita confirmada dos últimos 90 dias. */
export interface MixPagamento {
    participacao_cartao_bps: number
    amostra_vendas: number
}

export interface FornecedorFrete {
    fornecedor_id: string
    frete_tipico_bps: number
}

export interface PrecoConcorrencia {
    id: string
    concorrente: string | null
    preco_centavos: number
    observado_em: string
}

export interface Elasticidade {
    coeficiente: number
    variacao_preco_pct: number
    variacao_vendas_pct: number
}

export function listarMargensCategoria(apiFetch: ApiFetch) {
    return apiFetch<{ margens: CategoriaMargem[] }>('/catalogo/margens-categoria')
}

export function definirMargemCategoria(apiFetch: ApiFetch, margem: CategoriaMargem) {
    return apiFetch('/catalogo/margens-categoria', { method: 'PUT', body: margem })
}

export function removerMargemCategoria(apiFetch: ApiFetch, categoria: string) {
    return apiFetch(`/catalogo/margens-categoria/${encodeURIComponent(categoria)}`, { method: 'DELETE' })
}

export function listarCategorias(apiFetch: ApiFetch) {
    return apiFetch<{ categorias: string[] }>('/catalogo/categorias')
}

export function listarPrecificacaoProdutos(apiFetch: ApiFetch) {
    return apiFetch<{ produtos: ProdutoPrecificacao[] }>('/catalogo/precificacao-produtos')
}

/** Todos os campos vazios = remove os overrides (volta a categoria/padrão). */
export function definirPrecificacaoProduto(
    apiFetch: ApiFetch,
    produtoId: string,
    overrides: Omit<ProdutoPrecificacao, 'produto_id'>,
) {
    return apiFetch(`/catalogo/produtos/${produtoId}/precificacao`, { method: 'PUT', body: overrides })
}

export function listarGiroProdutos(apiFetch: ApiFetch) {
    return apiFetch<{ produtos: GiroProduto[] }>('/catalogo/giro-produtos')
}

export function obterMixPagamento(apiFetch: ApiFetch) {
    return apiFetch<{ mix: MixPagamento }>('/catalogo/mix-pagamento')
}

export function listarMaquinasCartao(apiFetch: ApiFetch) {
    return apiFetch<{ maquinas: MaquinaCartao[] }>('/catalogo/maquinas-cartao')
}

export function definirMaquinaCartao(apiFetch: ApiFetch, maquina: MaquinaCartao) {
    return apiFetch('/catalogo/maquinas-cartao', { method: 'PUT', body: maquina })
}

export function removerMaquinaCartao(apiFetch: ApiFetch, nome: string) {
    return apiFetch(`/catalogo/maquinas-cartao/${encodeURIComponent(nome)}`, { method: 'DELETE' })
}

export function listarFretesFornecedor(apiFetch: ApiFetch) {
    return apiFetch<{ fretes: FornecedorFrete[] }>('/catalogo/fretes-fornecedor')
}

/** frete_tipico_bps null = remove o frete típico do fornecedor. */
export function definirFreteFornecedor(apiFetch: ApiFetch, fornecedorId: string, freteTipicoBps: number | null) {
    return apiFetch(`/catalogo/fornecedores/${fornecedorId}/frete`, {
        method: 'PUT',
        body: { frete_tipico_bps: freteTipicoBps },
    })
}

export function listarPrecosConcorrencia(apiFetch: ApiFetch, produtoId: string) {
    return apiFetch<{ precos: PrecoConcorrencia[] }>(`/catalogo/produtos/${produtoId}/precos-concorrencia`)
}

export function registrarPrecoConcorrencia(
    apiFetch: ApiFetch,
    produtoId: string,
    dados: { concorrente: string | null; preco_centavos: number },
) {
    return apiFetch(`/catalogo/produtos/${produtoId}/precos-concorrencia`, { method: 'POST', body: dados })
}

export function removerPrecoConcorrencia(apiFetch: ApiFetch, produtoId: string, precoId: string) {
    return apiFetch(`/catalogo/produtos/${produtoId}/precos-concorrencia/${precoId}`, { method: 'DELETE' })
}

export function obterElasticidade(apiFetch: ApiFetch, produtoId: string) {
    return apiFetch<{ elasticidade: Elasticidade | null }>(`/catalogo/produtos/${produtoId}/elasticidade`)
}
