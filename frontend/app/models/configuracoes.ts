import type { ApiFetch } from './shared'

export interface Configuracoes {
    permite_orcamento_sem_estoque: boolean
    /** Dias até vendas/orçamentos não concretizados irem para a lixeira
     * (null = arquivamento automático desligado). */
    arquivamento_dias: number | null
    cnpj: string | null
    telefone: string | null
    endereco: string | null
    chave_pix: string | null
    informacoes_adicionais: string | null
    /** Percentuais em basis points (4000 = 40%); tudo opcional. */
    margem_padrao_bps: number | null
    imposto_venda_bps: number | null
    comissao_venda_bps: number | null
    taxa_cartao_bps: number | null
    frete_venda_bps: number | null
    outras_despesas_venda_bps: number | null
    custos_fixos_mensais_centavos: number | null
    vendas_mensais_estimadas: number | null
    /** Denominador do rateio proporcional: custos fixos ÷ faturamento = % do preço. */
    faturamento_mensal_estimado_centavos: number | null
    /** Alvo de crescimento (dashboard + score); não mexe nos preços. */
    meta_faturamento_mensal_centavos: number | null
}

/** Custo fixo mensal discriminado (aluguel, salário, DAS…). A soma dos itens
 * é mantida pelo backend em custos_fixos_mensais_centavos. */
export interface CustoFixo {
    nome: string
    valor_centavos: number
}

export function obterConfiguracoes(apiFetch: ApiFetch) {
    return apiFetch<Configuracoes>('/configuracoes')
}

export function atualizarConfiguracoes(apiFetch: ApiFetch, dados: Configuracoes) {
    return apiFetch('/configuracoes', { method: 'PUT', body: dados })
}

/** Perfil fiscal do tenant (reforma tributária): alimenta o motor de cálculo
 * de impostos na emissão de NF. Regime null = motor no comportamento legado. */
export interface PerfilFiscal {
    regime_tributario: string | null
    uf: string | null
    codigo_municipio: string | null
    crt: number | null
    ibs_cbs_regime_regular: boolean
    /** Alíquota efetiva do DAS em bps (700 = 7%) — anexo/faixa do Simples;
     * vira o custo tributário do vendedor quando o Simples está configurado. */
    aliquota_das_bps: number | null
}

export function obterPerfilFiscal(apiFetch: ApiFetch) {
    return apiFetch<PerfilFiscal>('/configuracoes/perfil-fiscal')
}

export function atualizarPerfilFiscal(apiFetch: ApiFetch, perfil: PerfilFiscal) {
    return apiFetch('/configuracoes/perfil-fiscal', { method: 'PUT', body: perfil })
}

export function listarCustosFixos(apiFetch: ApiFetch) {
    return apiFetch<{ custos: CustoFixo[] }>('/configuracoes/custos-fixos')
}

export function definirCustoFixo(apiFetch: ApiFetch, custo: CustoFixo) {
    return apiFetch('/configuracoes/custos-fixos', { method: 'PUT', body: custo })
}

export function removerCustoFixo(apiFetch: ApiFetch, nome: string) {
    return apiFetch(`/configuracoes/custos-fixos/${encodeURIComponent(nome)}`, { method: 'DELETE' })
}
