import type { NotaFiscal } from '~/models/fiscal'
import { cancelarNotaFiscal, listarNotasFiscais, retransmitirNotaFiscal } from '~/models/fiscal'

/** ViewModel da página de Notas Fiscais: concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useFiscalViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { notifySuccess, notifyError } = useNotify()

    const notas = ref<NotaFiscal[]>([])
    const loading = ref(false)
    const filters = ref({ global: { value: null as string | null, matchMode: 'contains' } })

    /** Tributos com valor > 0 na nota, prontos para exibição compacta.
     * Notas anteriores ao motor tributário retornam lista vazia (valores 0). */
    function impostosDaNota(nf: NotaFiscal): { sigla: string, centavos: number }[] {
        return [
            { sigla: 'ICMS', centavos: nf.icms_centavos },
            { sigla: 'PIS', centavos: nf.pis_centavos },
            { sigla: 'COFINS', centavos: nf.cofins_centavos },
            { sigla: 'ISS', centavos: nf.iss_centavos },
            { sigla: 'CBS', centavos: nf.cbs_centavos },
            { sigla: 'IBS UF', centavos: nf.ibs_uf_centavos },
            { sigla: 'IBS Mun', centavos: nf.ibs_mun_centavos },
            { sigla: 'IS', centavos: nf.is_centavos },
        ].filter(t => t.centavos > 0)
    }

    function statusSeverity(status: string) {
        return (
            {
                Autorizada: 'success',
                Transmitida: 'info',
                Gerada: 'secondary',
                Rejeitada: 'warn',
                Cancelada: 'danger',
            }[status] ?? 'secondary'
        )
    }

    async function carregar() {
        loading.value = true
        try {
            const { notas: lista } = await listarNotasFiscais(apiFetch)
            notas.value = lista
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    async function retransmitir(nf: NotaFiscal) {
        try {
            await retransmitirNotaFiscal(apiFetch, nf.nf_id)
            notifySuccess('OK', 'Nota retransmitida.')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Cancelamento ---
    const cancelarVisible = ref(false)
    const notaCancelar = ref<NotaFiscal | null>(null)
    const motivo = ref('')

    function abrirCancelar(nf: NotaFiscal) {
        notaCancelar.value = nf
        motivo.value = ''
        cancelarVisible.value = true
    }

    async function cancelar() {
        if (!notaCancelar.value) return
        try {
            await cancelarNotaFiscal(apiFetch, notaCancelar.value.nf_id, motivo.value)
            cancelarVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    return reactive({
        notas,
        loading,
        filters,
        impostosDaNota,
        statusSeverity,
        carregar,
        retransmitir,
        cancelarVisible,
        notaCancelar,
        motivo,
        abrirCancelar,
        cancelar,
    })
}
