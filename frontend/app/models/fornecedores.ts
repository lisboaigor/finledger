import type { ApiFetch } from './shared'

export interface Fornecedor {
    fornecedor_id: string
    razao_social: string
    cnpj: string
    ativo: boolean
}

export interface FornecedorPayload {
    razao_social: string
    cnpj?: string
    telefone: string | null
    email: string | null
    prazo_pagamento_dias: number
}

export function listarFornecedores(apiFetch: ApiFetch) {
    return apiFetch<{ fornecedores: Fornecedor[] }>('/fornecedores')
}

export function criarFornecedor(apiFetch: ApiFetch, payload: FornecedorPayload) {
    return apiFetch('/fornecedores', { method: 'POST', body: payload })
}

export function atualizarFornecedor(apiFetch: ApiFetch, fornecedorId: string, payload: FornecedorPayload) {
    return apiFetch(`/fornecedores/${fornecedorId}`, { method: 'PUT', body: payload })
}

export function alternarAtivoFornecedor(apiFetch: ApiFetch, fornecedorId: string, acao: 'desativar' | 'reativar') {
    return apiFetch(`/fornecedores/${fornecedorId}/${acao}`, { method: 'POST' })
}
