import type { ApiFetch } from './shared'

export interface Saldo {
    produto_id: string
    quantidade: number
    custo_medio: number
    estoque_minimo: number
}

export interface EntradaPayload {
    produto_id: string
    quantidade: number
    custo_unitario_centavos: number
    motivo: string
    nota_fiscal: string | null
}

export interface AjustePayload {
    quantidade_nova: number
    justificativa: string
}

export function listarSaldos(apiFetch: ApiFetch) {
    return apiFetch<{ saldos: Saldo[] }>('/estoque')
}

export function registrarEntrada(apiFetch: ApiFetch, payload: EntradaPayload) {
    return apiFetch('/estoque/entradas', { method: 'POST', body: payload })
}

export function ajustarSaldo(apiFetch: ApiFetch, produtoId: string, payload: AjustePayload) {
    return apiFetch(`/estoque/${produtoId}/ajuste`, { method: 'POST', body: payload })
}

export function definirMinimo(apiFetch: ApiFetch, produtoId: string, estoqueMinimo: number) {
    return apiFetch(`/estoque/${produtoId}/minimo`, { method: 'PUT', body: { estoque_minimo: estoqueMinimo } })
}
