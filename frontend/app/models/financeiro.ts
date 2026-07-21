import type { ApiFetch } from './shared'

export interface ContaReceber {
    conta_id: string
    venda_id: string
    cliente_id: string | null
    valor_original: number
    valor_recebido: number
    status: string
}

export interface ContaPagar {
    conta_id: string
    pedido_id: string
    fornecedor_id: string
    valor_original: number
    valor_pago: number
    status: string
}

export function listarContasReceber(apiFetch: ApiFetch) {
    return apiFetch<{ contas: ContaReceber[] }>('/financeiro/contas-receber')
}

export function listarContasPagar(apiFetch: ApiFetch) {
    return apiFetch<{ contas: ContaPagar[] }>('/financeiro/contas-pagar')
}

export function registrarPagamentoReceber(apiFetch: ApiFetch, contaId: string, valorCentavos: number) {
    return apiFetch(`/financeiro/contas-receber/${contaId}/pagamento`, {
        method: 'POST',
        body: { valor_centavos: valorCentavos },
    })
}

export function registrarPagamentoPagar(apiFetch: ApiFetch, contaId: string, valorCentavos: number) {
    return apiFetch(`/financeiro/contas-pagar/${contaId}/pagamento`, {
        method: 'POST',
        body: { valor_centavos: valorCentavos },
    })
}

export function estornarContaReceber(apiFetch: ApiFetch, contaId: string, motivo: string) {
    return apiFetch(`/financeiro/contas-receber/${contaId}/estornar`, {
        method: 'POST',
        body: { motivo },
    })
}
