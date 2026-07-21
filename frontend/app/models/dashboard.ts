import type { ApiFetch } from './shared'

/** Projeções enxutas usadas apenas pelo dashboard — não são os tipos completos
 * de cada módulo (ver models/vendas.ts, models/orcamentos.ts etc. para isso). */

export interface VendaResumo {
    venda_id: string
    cliente_id: string | null
    status: string
    total_centavos: number
    forma_pagamento: string | null
}

export interface OrcamentoResumo {
    orcamento_id: string
    cliente_id: string | null
    status: string
    total_centavos: number
}

export interface NotaResumo {
    nf_id: string
    cliente_id: string | null
    numero: number
    status: string
    total_centavos: number
}

export interface SaldoResumo {
    produto_id: string
    quantidade: number
    estoque_minimo: number
}

export interface ProdutoResumo {
    produto_id: string
    sku: string
    descricao: string
    ativo: boolean
}

export interface ClienteResumo {
    cliente_id: string
    nome: string
}

export interface ContaResumo {
    valor_original: number
    valor_recebido?: number
    valor_pago?: number
}

export function listarProdutosResumo(apiFetch: ApiFetch) {
    return apiFetch<{ produtos: ProdutoResumo[] }>('/catalogo/produtos')
}

export function listarClientesResumo(apiFetch: ApiFetch) {
    return apiFetch<{ clientes: ClienteResumo[] }>('/crm/clientes')
}

export function listarVendasResumo(apiFetch: ApiFetch) {
    return apiFetch<{ vendas: VendaResumo[] }>('/vendas')
}

export function listarOrcamentosResumo(apiFetch: ApiFetch) {
    return apiFetch<{ orcamentos: OrcamentoResumo[] }>('/orcamentos')
}

export function listarContasReceberResumo(apiFetch: ApiFetch) {
    return apiFetch<{ contas: ContaResumo[] }>('/financeiro/contas-receber')
}

export function listarContasPagarResumo(apiFetch: ApiFetch) {
    return apiFetch<{ contas: ContaResumo[] }>('/financeiro/contas-pagar')
}

export function listarSaldosResumo(apiFetch: ApiFetch) {
    return apiFetch<{ saldos: SaldoResumo[] }>('/estoque')
}

export function listarNotasResumo(apiFetch: ApiFetch) {
    return apiFetch<{ notas: NotaResumo[] }>('/fiscal/notas')
}
