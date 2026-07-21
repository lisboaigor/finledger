import type { Admin } from '~/models/backoffice'
import {
    changeAdminPassword,
    criarAdmin as criarAdminModel,
    desativarAdmin,
    listarAdmins,
    reativarAdmin,
    salvarPermissoesAdmin,
} from '~/models/backoffice'

const todasPermissoes = [
    { label: 'Listar tenants', value: 'tenants:read' },
    { label: 'Criar/suspender/reativar tenants', value: 'tenants:write' },
    { label: 'Acessar como tenant (suporte)', value: 'tenants:impersonate' },
    { label: 'Gerenciar admins', value: 'admins:manage' },
]

/** ViewModel da página de Admins de Suporte (backoffice): concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useAdminsViewModel() {
    const { apiFetch, apiErrorMessage } = useBackofficeApi()
    const { hasPermission, token } = useBackofficeAuth()
    const { notifySuccess, notifyError } = useNotify()

    function verificarAcesso() {
        if (!token.value) return navigateTo('/login')
        if (!hasPermission('admins:manage')) return navigateTo('/tenants')
    }

    const admins = ref<Admin[]>([])
    const loading = ref(false)
    const filters = ref({ global: { value: null as string | null, matchMode: 'contains' } })

    async function carregar() {
        loading.value = true
        try {
            const { admins: a } = await listarAdmins(apiFetch)
            admins.value = a
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    async function desativar(id: string) {
        try {
            await desativarAdmin(apiFetch, id)
            notifySuccess('Desativado')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    async function reativar(id: string) {
        try {
            await reativarAdmin(apiFetch, id)
            notifySuccess('Reativado')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Novo admin ---
    const novoVisible = ref(false)
    const saving = ref(false)
    const novoForm = reactive({ username: '', senha: '', permissions: [] as string[] })

    async function criarAdmin() {
        saving.value = true
        try {
            await criarAdminModel(apiFetch, novoForm)
            novoVisible.value = false
            Object.assign(novoForm, { username: '', senha: '', permissions: [] })
            notifySuccess('Admin criado')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            saving.value = false
        }
    }

    // --- Reset password ---
    const passwordVisible = ref(false)
    const passwordForm = reactive({ user_id: '', username: '', password: '' })

    function openPasswordReset(admin: Admin) {
        passwordForm.user_id = admin.user_id
        passwordForm.username = admin.username
        passwordForm.password = ''
        passwordVisible.value = true
    }

    async function savePassword() {
        saving.value = true
        try {
            await changeAdminPassword(apiFetch, passwordForm.user_id, passwordForm.password)
            passwordVisible.value = false
            notifySuccess('Senha redefinida')
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            saving.value = false
        }
    }

    // --- Editar permissões ---
    const permissoesVisible = ref(false)
    const permissoesForm = reactive({ user_id: '', permissions: [] as string[] })

    function abrirPermissoes(admin: Admin) {
        permissoesForm.user_id = admin.user_id
        permissoesForm.permissions = [...admin.permissions]
        permissoesVisible.value = true
    }

    async function salvarPermissoes() {
        saving.value = true
        try {
            await salvarPermissoesAdmin(apiFetch, permissoesForm.user_id, permissoesForm.permissions)
            permissoesVisible.value = false
            notifySuccess('Permissões atualizadas')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            saving.value = false
        }
    }

    return reactive({
        todasPermissoes,
        verificarAcesso,
        admins,
        loading,
        filters,
        carregar,
        desativar,
        reativar,
        novoVisible,
        saving,
        novoForm,
        criarAdmin,
        permissoesVisible,
        permissoesForm,
        abrirPermissoes,
        salvarPermissoes,
        passwordVisible,
        passwordForm,
        openPasswordReset,
        savePassword,
    })
}
