import type { ApiFetch } from './shared'

export interface Orcamento {
    orcamento_id: string
    vendedor_id: string
    cliente_id: string | null
    /** Nome informal do cliente quando não há cadastro completo no CRM (atendimento de balcão). */
    cliente_avulso: string | null
    total_centavos: number
    desconto_centavos: number
    status: string
    validade_dias: number
}

export interface OrcamentoItem {
    item_id: string
    produto_id: string
    sku: string
    descricao: string
    quantidade: number
    preco_unitario_centavos: number
}

export interface OrcamentoDetalhes {
    orcamento: Orcamento
    itens: OrcamentoItem[]
}

export interface NovoItemPayload {
    produto_id: string
    sku: string
    descricao: string
    quantidade: number
    preco_unitario_centavos: number
}

export interface CabecalhoPayload {
    cliente_id: string | null
    cliente_avulso: string | null
    validade_dias: number
}

/** Orçamento na lixeira (arquivado pela rotina de limpeza — nunca excluído). */
export interface OrcamentoArquivado extends Orcamento {
    criado_em: string
    arquivado_em: string
}

export function listarLixeiraOrcamentos(apiFetch: ApiFetch) {
    return apiFetch<{ orcamentos: OrcamentoArquivado[] }>('/orcamentos/lixeira')
}

export function restaurarOrcamento(apiFetch: ApiFetch, orcamentoId: string) {
    return apiFetch(`/orcamentos/${orcamentoId}/restaurar`, { method: 'POST' })
}

export function listarOrcamentos(apiFetch: ApiFetch, opts?: { apenasAbertos?: boolean }) {
    return apiFetch<{ orcamentos: Orcamento[] }>('/orcamentos', {
        query: opts?.apenasAbertos ? { abertos: 'true' } : undefined,
    })
}

export function buscarOrcamento(apiFetch: ApiFetch, orcamentoId: string) {
    return apiFetch<OrcamentoDetalhes>(`/orcamentos/${orcamentoId}`)
}

export function criarOrcamento(apiFetch: ApiFetch, payload: CabecalhoPayload) {
    return apiFetch<{ orcamento_id: string }>('/orcamentos', { method: 'POST', body: payload })
}

export function atualizarCabecalho(apiFetch: ApiFetch, orcamentoId: string, payload: CabecalhoPayload) {
    return apiFetch(`/orcamentos/${orcamentoId}`, { method: 'PUT', body: payload })
}

export function adicionarItem(apiFetch: ApiFetch, orcamentoId: string, item: NovoItemPayload) {
    return apiFetch(`/orcamentos/${orcamentoId}/itens`, { method: 'POST', body: item })
}

export function removerItem(apiFetch: ApiFetch, orcamentoId: string, itemId: string) {
    return apiFetch(`/orcamentos/${orcamentoId}/itens/${itemId}`, { method: 'DELETE' })
}

export function aplicarDesconto(apiFetch: ApiFetch, orcamentoId: string, descontoCentavos: number) {
    return apiFetch(`/orcamentos/${orcamentoId}/desconto`, {
        method: 'POST',
        body: { desconto_centavos: descontoCentavos },
    })
}

export function emitirOrcamento(apiFetch: ApiFetch, orcamentoId: string) {
    return apiFetch(`/orcamentos/${orcamentoId}/emitir`, { method: 'POST' })
}

export function aceitarOrcamento(apiFetch: ApiFetch, orcamentoId: string) {
    return apiFetch(`/orcamentos/${orcamentoId}/aceitar`, { method: 'POST' })
}

export function recusarOrcamento(apiFetch: ApiFetch, orcamentoId: string, motivo: string) {
    return apiFetch(`/orcamentos/${orcamentoId}/recusar`, { method: 'POST', body: { motivo } })
}

export function cancelarOrcamento(apiFetch: ApiFetch, orcamentoId: string, motivo: string) {
    return apiFetch(`/orcamentos/${orcamentoId}/cancelar`, { method: 'POST', body: { motivo } })
}
