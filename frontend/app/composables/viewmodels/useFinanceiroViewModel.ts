import type { ContaPagar, ContaReceber } from '~/models/financeiro'
import {
    estornarContaReceber,
    listarContasPagar,
    listarContasReceber,
    registrarPagamentoPagar,
    registrarPagamentoReceber,
} from '~/models/financeiro'

/** ViewModel da página Financeiro: concentra estado e regras de negócio de
 * contas a receber e a pagar; a View só lê estado e dispara ações. */
export function useFinanceiroViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { toCentavos } = useFormat()
    const { notifySuccess, notifyError } = useNotify()

    const receber = ref<ContaReceber[]>([])
    const pagar = ref<ContaPagar[]>([])
    const loading = ref(false)
    const filtersReceber = ref({ global: { value: null as string | null, matchMode: 'contains' } })
    const filtersPagar = ref({ global: { value: null as string | null, matchMode: 'contains' } })

    function statusSeverity(status: string) {
        return (
            {
                Liquidada: 'success',
                Parcial: 'warn',
                Pendente: 'info',
                Estornada: 'danger',
            }[status] ?? 'secondary'
        )
    }
    function saldoReceber(c: ContaReceber) {
        return c.valor_original - c.valor_recebido
    }
    function saldoPagar(c: ContaPagar) {
        return c.valor_original - c.valor_pago
    }

    async function carregar() {
        loading.value = true
        try {
            const [{ contas: cr }, { contas: cp }] = await Promise.all([
                listarContasReceber(apiFetch),
                listarContasPagar(apiFetch),
            ])
            receber.value = cr
            pagar.value = cp
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    const totalReceber = computed(() => receber.value.reduce((s, c) => s + saldoReceber(c), 0))
    const totalPagar = computed(() => pagar.value.reduce((s, c) => s + saldoPagar(c), 0))

    // --- Pagamento (receber ou pagar) ---
    const pagVisible = ref(false)
    const pagTipo = ref<'receber' | 'pagar'>('receber')
    const pagConta = ref<ContaReceber | ContaPagar | null>(null)
    const pagValor = ref(0)

    function abrirPagamento(tipo: 'receber' | 'pagar', conta: ContaReceber | ContaPagar) {
        pagTipo.value = tipo
        pagConta.value = conta
        pagValor.value =
            (tipo === 'receber'
                ? saldoReceber(conta as ContaReceber)
                : saldoPagar(conta as ContaPagar)) / 100
        pagVisible.value = true
    }

    async function confirmarPagamento() {
        if (!pagConta.value) return
        try {
            if (pagTipo.value === 'receber') {
                await registrarPagamentoReceber(apiFetch, pagConta.value.conta_id, toCentavos(pagValor.value))
            } else {
                await registrarPagamentoPagar(apiFetch, pagConta.value.conta_id, toCentavos(pagValor.value))
            }
            notifySuccess('Registrado', 'Pagamento registrado.')
            pagVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Estorno (apenas receber) ---
    const estornoVisible = ref(false)
    const estornoConta = ref<ContaReceber | null>(null)
    const motivoEstorno = ref('')

    function abrirEstorno(conta: ContaReceber) {
        estornoConta.value = conta
        motivoEstorno.value = ''
        estornoVisible.value = true
    }
    async function estornar() {
        if (!estornoConta.value) return
        try {
            await estornarContaReceber(apiFetch, estornoConta.value.conta_id, motivoEstorno.value)
            estornoVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    return reactive({
        receber,
        pagar,
        loading,
        filtersReceber,
        filtersPagar,
        statusSeverity,
        saldoReceber,
        saldoPagar,
        carregar,
        totalReceber,
        totalPagar,
        pagVisible,
        pagTipo,
        pagConta,
        pagValor,
        abrirPagamento,
        confirmarPagamento,
        estornoVisible,
        estornoConta,
        motivoEstorno,
        abrirEstorno,
        estornar,
    })
}
