import type { ApiFetch } from './shared'

export interface NotaFiscal {
    nf_id: string
    venda_id: string
    cliente_id: string | null
    modelo: string
    serie: string
    numero: number
    chave: string | null
    status: string
    total_centavos: number
    /** Devolução registrada antes da integração SEFAZ — cancelamento aguardando. */
    cancelamento_pendente: boolean
}

export function listarNotasFiscais(apiFetch: ApiFetch) {
    return apiFetch<{ notas: NotaFiscal[] }>('/fiscal/notas')
}

export function retransmitirNotaFiscal(apiFetch: ApiFetch, nfId: string) {
    return apiFetch(`/fiscal/notas/${nfId}/retransmitir`, { method: 'POST' })
}

export function cancelarNotaFiscal(apiFetch: ApiFetch, nfId: string, motivo: string) {
    return apiFetch(`/fiscal/notas/${nfId}/cancelar`, { method: 'POST', body: { motivo } })
}
