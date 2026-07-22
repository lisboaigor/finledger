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
    /** Total da nota (produtos − desconto). */
    total_centavos: number
    /** Desconto global da venda destacado na NF (0 em notas antigas). */
    desconto_centavos: number
    /** Devolução registrada antes da integração SEFAZ — cancelamento aguardando. */
    cancelamento_pendente: boolean
    // Breakdown de impostos (reforma tributária) — notas antigas têm 0.
    icms_centavos: number
    pis_centavos: number
    cofins_centavos: number
    iss_centavos: number
    cbs_centavos: number
    ibs_uf_centavos: number
    ibs_mun_centavos: number
    is_centavos: number
}

export interface ClasseTributaria {
    c_class_trib: string
    descricao: string
    cst_ibs_cbs: string
    reducao_bps: number
}

export function listarClassesTributarias(apiFetch: ApiFetch) {
    return apiFetch<{ classes: ClasseTributaria[] }>('/fiscal/classes-tributarias')
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
