import type { ApiFetch } from './shared'

export interface Pedido {
    pedido_id: string
    comprador_id: string
    fornecedor_id: string
    total_centavos: number
    prazo_pagamento_dias: number
    status: string
}

export interface PedidoItem {
    produto_id: string
    quantidade: number
    custo_unitario_centavos: number
}

export interface PedidoDetalhes {
    pedido: Pedido
    itens: PedidoItem[]
}

export interface NovoItemPedidoPayload {
    produto_id: string
    quantidade: number
    custo_unitario_centavos: number
}

export interface GerarPedidoPayload {
    fornecedor_id: string
    prazo_pagamento_dias: number
    itens: NovoItemPedidoPayload[]
}

export interface ReceberPayload {
    itens_recebidos: { produto_id: string; quantidade: number }[]
}

export function listarPedidos(apiFetch: ApiFetch) {
    return apiFetch<{ pedidos: Pedido[] }>('/compras/pedidos')
}

export function buscarPedido(apiFetch: ApiFetch, pedidoId: string) {
    return apiFetch<PedidoDetalhes>(`/compras/pedidos/${pedidoId}`)
}

export function gerarPedido(apiFetch: ApiFetch, payload: GerarPedidoPayload) {
    return apiFetch('/compras/pedidos', { method: 'POST', body: payload })
}

export function aprovarPedido(apiFetch: ApiFetch, pedidoId: string) {
    return apiFetch(`/compras/pedidos/${pedidoId}/aprovar`, { method: 'POST' })
}

export function enviarPedido(apiFetch: ApiFetch, pedidoId: string) {
    return apiFetch(`/compras/pedidos/${pedidoId}/enviar`, { method: 'POST' })
}

export function receberPedido(apiFetch: ApiFetch, pedidoId: string, payload: ReceberPayload) {
    return apiFetch(`/compras/pedidos/${pedidoId}/receber`, { method: 'POST', body: payload })
}

export function cancelarPedido(apiFetch: ApiFetch, pedidoId: string, motivo: string) {
    return apiFetch(`/compras/pedidos/${pedidoId}/cancelar`, { method: 'POST', body: { motivo } })
}
