import type { ApiFetch } from './shared'

/** Contratos do módulo de BI prescritivo (`/bi/*`). */

export interface BiResumo {
    receita_mes_centavos: number
    receita_mes_anterior_centavos: number
    vencidas_centavos: number
    caixa_30d_centavos: number
    margem_percent: number | null
    /** Margem líquida de impostos da NF (reforma LC 214/2025); null sem vendas. */
    margem_liquida_percent: number | null
    conversao_percent: number | null
}

export interface ReceitaDia {
    dia: string
    total_centavos: number
}

/** Score de saúde do negócio (0–100) com o detalhamento de cada componente. */
export interface SaudeComponente {
    nome: string
    nota: number
    peso: number
    detalhe: string
}

export interface SaudeNegocio {
    /** null enquanto não há dado suficiente para nenhum componente. */
    score: number | null
    componentes: SaudeComponente[]
}

export interface BiAlerta {
    alerta_id: string
    codigo: string
    titulo: string
    mensagem: string
    link: string
    impacto_centavos: number
    urgencia_dias: number
    score: number
    status: string
}

// ── Análises (V2) ─────────────────────────────────────────────────────────────

export interface CicloFinanceiro {
    dso: number
    dio: number
    dpo: number
    ccc: number
}

export interface AgingFaixa {
    faixa: string
    quantidade: number
    total_centavos: number
}

export interface SemanaFluxo {
    semana: string
    receber_centavos: number
    pagar_centavos: number
}

export interface Devedor {
    cliente_id: string | null
    nome: string
    saldo_centavos: number
    dias_atraso: number
}

export interface FinanceiroBi {
    ciclo: CicloFinanceiro
    aging: AgingFaixa[]
    projecao: SemanaFluxo[]
    devedores: Devedor[]
}

export interface FunilEtapa {
    status: string
    quantidade: number
    total_centavos: number
}

export interface OrcamentoExpirando {
    orcamento_id: string
    cliente: string
    total_centavos: number
    vence_em_dias: number
}

export interface VendedorDesempenho {
    vendedor: string
    receita_centavos: number
    vendas: number
    ticket_centavos: number
    conversao_percent: number | null
    desconto_percent: number | null
}

export interface RfmSegmento {
    segmento: string
    clientes: number
    valor_centavos: number
}

export interface ClienteRisco {
    cliente_id: string
    nome: string
    valor_12m_centavos: number
    recencia_dias: number
    telefone: string | null
    email: string | null
}

export interface ComercialBi {
    funil: FunilEtapa[]
    expirando: OrcamentoExpirando[]
    vendedores: VendedorDesempenho[]
    rfm: RfmSegmento[]
    em_risco: ClienteRisco[]
}

export interface MatrizCelula {
    abc: string
    xyz: string
    produtos: number
    valor_centavos: number
}

export interface Ruptura {
    produto_id: string
    sku: string
    descricao: string
    classe_abc: string
    quantidade: number
    cobertura_dias: number | null
    sugestao_compra: number
}

export interface EstoqueMorto {
    produto_id: string
    sku: string
    descricao: string
    quantidade: number
    valor_centavos: number
    dias_sem_venda: number | null
}

export interface CategoriaGiro {
    categoria: string
    receita_centavos: number
    margem_percent: number | null
    valor_estoque_centavos: number
    giro: number | null
}

export interface PedidoParado {
    pedido_id: string
    fornecedor: string
    total_centavos: number
    status: string
    dias_parado: number
}

export interface EstoqueBi {
    matriz: MatrizCelula[]
    rupturas: Ruptura[]
    mortos: EstoqueMorto[]
    categorias: CategoriaGiro[]
    pedidos_parados: PedidoParado[]
}

export function obterFinanceiroBi(apiFetch: ApiFetch) {
    return apiFetch<FinanceiroBi>('/bi/financeiro')
}

export function obterComercialBi(apiFetch: ApiFetch) {
    return apiFetch<ComercialBi>('/bi/comercial')
}

export function obterEstoqueBi(apiFetch: ApiFetch) {
    return apiFetch<EstoqueBi>('/bi/estoque')
}

export function obterResumoBi(apiFetch: ApiFetch) {
    return apiFetch<{
        resumo: BiResumo
        receita_diaria: ReceitaDia[]
        saude: SaudeNegocio
        meta_faturamento_mensal_centavos: number | null
        /** Fim do último ciclo de ETL bem-sucedido (ISO); null antes do 1º ciclo. */
        etl_atualizado_em: string | null
    }>('/bi/resumo')
}

export function listarAlertasBi(apiFetch: ApiFetch, limite = 5) {
    return apiFetch<{ alertas: BiAlerta[] }>(`/bi/alertas?limite=${limite}`)
}

export function enviarFeedbackAlerta(apiFetch: ApiFetch, alertaId: string, acao: 'resolvido' | 'ignorado') {
    return apiFetch<void>(`/bi/alertas/${alertaId}/feedback`, {
        method: 'POST',
        body: { acao },
    })
}
