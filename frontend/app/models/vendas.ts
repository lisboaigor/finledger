import type { ApiFetch } from './shared'

export interface Venda {
    venda_id: string
    vendedor_id: string
    cliente_id: string | null
    total_centavos: number
    status: string
    forma_pagamento: string | null
}

export interface VendaItem {
    item_id: string
    produto_id: string
    sku: string
    descricao: string
    quantidade: number
    preco_unitario_centavos: number
}

export interface VendaDetalhes {
    venda: Venda
    itens: VendaItem[]
}

export interface NovoItemVendaPayload {
    produto_id: string
    sku: string
    descricao: string
    quantidade: number
    preco_unitario_centavos: number
    /** Confirmação explícita do vendedor para vender acima do saldo em
     * estoque (venda sob encomenda). */
    vender_sem_estoque?: boolean
}

/** Venda na lixeira (arquivada pela rotina de limpeza — nunca excluída). */
export interface VendaArquivada extends Venda {
    criada_em: string
    arquivada_em: string
}

export function listarLixeiraVendas(apiFetch: ApiFetch) {
    return apiFetch<{ vendas: VendaArquivada[] }>('/vendas/lixeira')
}

export function restaurarVenda(apiFetch: ApiFetch, vendaId: string) {
    return apiFetch(`/vendas/${vendaId}/restaurar`, { method: 'POST' })
}

export function listarVendas(apiFetch: ApiFetch, opts?: { produtoBusca?: string, apenasAbertas?: boolean }) {
    const query: Record<string, string> = {}
    if (opts?.produtoBusca) query.produto = opts.produtoBusca
    if (opts?.apenasAbertas) query.abertas = 'true'
    return apiFetch<{ vendas: Venda[] }>('/vendas', {
        query: Object.keys(query).length ? query : undefined,
    })
}

export function buscarVenda(apiFetch: ApiFetch, vendaId: string) {
    return apiFetch<VendaDetalhes>(`/vendas/${vendaId}`)
}

export function iniciarVenda(apiFetch: ApiFetch, clienteId: string | null) {
    return apiFetch<{ venda_id: string }>('/vendas', { method: 'POST', body: { cliente_id: clienteId } })
}

export function atualizarClienteVenda(apiFetch: ApiFetch, vendaId: string, clienteId: string | null) {
    return apiFetch(`/vendas/${vendaId}`, { method: 'PUT', body: { cliente_id: clienteId } })
}

export function adicionarItemVenda(apiFetch: ApiFetch, vendaId: string, item: NovoItemVendaPayload) {
    return apiFetch(`/vendas/${vendaId}/itens`, { method: 'POST', body: item })
}

export function removerItemVenda(apiFetch: ApiFetch, vendaId: string, itemId: string) {
    return apiFetch(`/vendas/${vendaId}/itens/${itemId}`, { method: 'DELETE' })
}

export function definirFormaPagamento(apiFetch: ApiFetch, vendaId: string, forma: unknown) {
    return apiFetch(`/vendas/${vendaId}/forma-pagamento`, { method: 'POST', body: { forma } })
}

export function confirmarVenda(apiFetch: ApiFetch, vendaId: string) {
    return apiFetch(`/vendas/${vendaId}/confirmar`, { method: 'POST' })
}

export function cancelarVenda(apiFetch: ApiFetch, vendaId: string, motivo: string) {
    return apiFetch(`/vendas/${vendaId}/cancelar`, { method: 'POST', body: { motivo } })
}

export interface DevolucaoItemPayload {
    item_id: string
    quantidade: number
}

/** Devolução parcial ajusta a venda (e a NF será reemitida quando a integração
 * SEFAZ estiver ativa); devolução total desfaz a venda. Estoque reentra sempre. */
export function devolverItensVenda(
    apiFetch: ApiFetch,
    vendaId: string,
    itens: DevolucaoItemPayload[],
    motivo: string,
) {
    return apiFetch(`/vendas/${vendaId}/devolver`, { method: 'POST', body: { itens, motivo } })
}
