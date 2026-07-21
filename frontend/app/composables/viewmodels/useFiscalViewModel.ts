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
