import type { Cliente } from '~/models/crm'
import {
    alternarAtivoCliente,
    atualizarCliente,
    bloquearCliente,
    criarCliente,
    desbloquearCliente,
    listarClientes,
} from '~/models/crm'

/** ViewModel da página de Clientes: concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useClientesViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { isAdmin } = useAuth()
    const { notifySuccess, notifyError } = useNotify()
    const { buscarAproximado } = useFuzzySearch()
    const confirm = useConfirm()

    const clientes = ref<Cliente[]>([])
    const loading = ref(false)
    const filtro = ref('')

    const filtrados = computed(() =>
        buscarAproximado(clientes.value, filtro.value, (c) => `${c.nome} ${c.cpf_cnpj} ${c.email ?? ''}`),
    )

    async function carregar() {
        loading.value = true
        try {
            const { clientes: lista } = await listarClientes(apiFetch)
            clientes.value = lista
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    // --- Cadastro / edição ---
    const dialogVisible = ref(false)
    const editando = ref<Cliente | null>(null)
    const salvando = ref(false)
    const form = reactive({ nome: '', cpf_cnpj: '', telefone: '', email: '' })

    function abrirNovo() {
        editando.value = null
        Object.assign(form, { nome: '', cpf_cnpj: '', telefone: '', email: '' })
        dialogVisible.value = true
    }

    function abrirEdicao(c: Cliente) {
        editando.value = c
        Object.assign(form, {
            nome: c.nome,
            cpf_cnpj: c.cpf_cnpj,
            telefone: c.telefone ?? '',
            email: c.email ?? '',
        })
        dialogVisible.value = true
    }

    async function salvar() {
        salvando.value = true
        try {
            if (editando.value) {
                await atualizarCliente(apiFetch, editando.value.cliente_id, {
                    nome: form.nome,
                    telefone: form.telefone || null,
                    email: form.email || null,
                })
            } else {
                await criarCliente(apiFetch, {
                    nome: form.nome,
                    cpf_cnpj: form.cpf_cnpj,
                    telefone: form.telefone || null,
                    email: form.email || null,
                })
            }
            notifySuccess('Salvo', 'Cliente salvo.')
            dialogVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvando.value = false
        }
    }

    // --- Bloqueio ---
    const bloqueioVisible = ref(false)
    const clienteBloqueio = ref<Cliente | null>(null)
    const motivo = ref('')

    function abrirBloqueio(c: Cliente) {
        clienteBloqueio.value = c
        motivo.value = ''
        bloqueioVisible.value = true
    }

    async function bloquear() {
        if (!clienteBloqueio.value) return
        try {
            await bloquearCliente(apiFetch, clienteBloqueio.value.cliente_id, motivo.value)
            bloqueioVisible.value = false
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    async function desbloquear(c: Cliente) {
        try {
            await desbloquearCliente(apiFetch, c.cliente_id)
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    async function alternarAtivo(c: Cliente) {
        try {
            await alternarAtivoCliente(apiFetch, c.cliente_id, c.ativo)
            notifySuccess(
                c.ativo ? 'Cliente excluído' : 'Cliente reativado',
                c.ativo ? 'Cliente desativado com sucesso.' : 'Cliente reativado com sucesso.',
            )
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Exclusão (desativação) ---
    function abrirExclusao(c: Cliente) {
        confirm.require({
            header: 'Excluir Cliente',
            message: `Tem certeza que deseja excluir o cliente ${c.nome}? O cliente ficará inativo, mas o histórico será preservado.`,
            acceptLabel: 'Excluir',
            rejectLabel: 'Cancelar',
            variant: 'danger',
            accept: () => alternarAtivo(c),
        })
    }

    return reactive({
        isAdmin,
        clientes,
        loading,
        filtro,
        filtrados,
        carregar,
        dialogVisible,
        editando,
        salvando,
        form,
        abrirNovo,
        abrirEdicao,
        salvar,
        bloqueioVisible,
        clienteBloqueio,
        motivo,
        abrirBloqueio,
        bloquear,
        desbloquear,
        alternarAtivo,
        abrirExclusao,
    })
}
