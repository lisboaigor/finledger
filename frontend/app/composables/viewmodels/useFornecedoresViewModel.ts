import type { Fornecedor } from '~/models/fornecedores'
import { alternarAtivoFornecedor, atualizarFornecedor, criarFornecedor, listarFornecedores } from '~/models/fornecedores'
import { definirFreteFornecedor, listarFretesFornecedor } from '~/models/catalogo'

/** ViewModel da página de Fornecedores: concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useFornecedoresViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { isAdmin } = useAuth()
    const { notifySuccess, notifyError } = useNotify()
    const { buscarAproximado } = useFuzzySearch()

    const fornecedores = ref<Fornecedor[]>([])
    const loading = ref(false)
    const filtro = ref('')

    const filtrados = computed(() =>
        buscarAproximado(fornecedores.value, filtro.value, (f) => `${f.razao_social} ${f.cnpj}`),
    )

    async function carregar() {
        loading.value = true
        try {
            const { fornecedores: lista } = await listarFornecedores(apiFetch)
            fornecedores.value = lista
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    // --- Novo / edição ---
    const dialogVisible = ref(false)
    const editando = ref<Fornecedor | null>(null)
    const salvando = ref(false)
    const form = reactive({
        razao_social: '',
        cnpj: '',
        telefone: '',
        email: '',
        prazo_pagamento_dias: 30,
    })

    /** Frete típico de compra (% sobre o valor da mercadoria) — pré-preenche
     * o frete da remessa na entrada de estoque. Config de precificação, não
     * faz parte do agregado Fornecedor. */
    const freteTipicoPct = ref<number | null>(null)
    const freteTipicoOriginal = ref<number | null>(null)
    const { invalidar: invalidarMargens } = useMargens()

    function abrirNovo() {
        editando.value = null
        Object.assign(form, {
            razao_social: '',
            cnpj: '',
            telefone: '',
            email: '',
            prazo_pagamento_dias: 30,
        })
        dialogVisible.value = true
    }

    function abrirEdicao(f: Fornecedor) {
        editando.value = f
        Object.assign(form, {
            razao_social: f.razao_social,
            cnpj: f.cnpj,
            telefone: '',
            email: '',
            prazo_pagamento_dias: 30,
        })
        freteTipicoPct.value = null
        freteTipicoOriginal.value = null
        void listarFretesFornecedor(apiFetch)
            .then(({ fretes }) => {
                const frete = fretes.find((x) => x.fornecedor_id === f.fornecedor_id)
                freteTipicoPct.value = frete ? frete.frete_tipico_bps / 100 : null
                freteTipicoOriginal.value = freteTipicoPct.value
            })
            .catch(() => {})
        dialogVisible.value = true
    }

    async function salvar() {
        salvando.value = true
        try {
            if (editando.value) {
                await atualizarFornecedor(apiFetch, editando.value.fornecedor_id, {
                    razao_social: form.razao_social,
                    telefone: form.telefone || null,
                    email: form.email || null,
                    prazo_pagamento_dias: form.prazo_pagamento_dias,
                })
                if (freteTipicoPct.value !== freteTipicoOriginal.value) {
                    await definirFreteFornecedor(
                        apiFetch,
                        editando.value.fornecedor_id,
                        freteTipicoPct.value == null ? null : Math.round(freteTipicoPct.value * 100),
                    )
                    invalidarMargens()
                }
            } else {
                await criarFornecedor(apiFetch, {
                    razao_social: form.razao_social,
                    cnpj: form.cnpj,
                    telefone: form.telefone || null,
                    email: form.email || null,
                    prazo_pagamento_dias: form.prazo_pagamento_dias,
                })
            }
            notifySuccess('Salvo', 'Fornecedor salvo.')
            dialogVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvando.value = false
        }
    }

    async function alternarAtivo(f: Fornecedor) {
        const acao = f.ativo ? 'desativar' : 'reativar'
        try {
            await alternarAtivoFornecedor(apiFetch, f.fornecedor_id, acao)
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    return reactive({
        isAdmin,
        fornecedores,
        loading,
        filtro,
        filtrados,
        carregar,
        dialogVisible,
        editando,
        salvando,
        form,
        freteTipicoPct,
        abrirNovo,
        abrirEdicao,
        salvar,
        alternarAtivo,
    })
}
