import { ROLE_LABELS, type RoleName } from '~/composables/useAuth'
import type { Usuario } from '~/models/identity'
import {
    alterarSenha as alterarSenhaModel,
    desativarUsuario,
    listarUsuarios,
    reativarUsuario,
    registrarUsuario,
    salvarRolesUsuario,
} from '~/models/identity'

const rolesOpcoes = Object.entries(ROLE_LABELS) as [RoleName, string][]

/** ViewModel da página de Usuários (identity, escopo de tenant): concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useUsuariosViewModel() {
    const { apiFetch, apiErrorMessage } = useApi()
    const { isAdmin, username } = useAuth()
    const { notifySuccess, notifyWarn, notifyError } = useNotify()

    const usuarios = ref<Usuario[]>([])
    const loading = ref(false)
    const filters = ref({ global: { value: null as string | null, matchMode: 'contains' } })

    // Backend stores roles as comma-separated string or JSON array
    function parseRoles(roles: string): string[] {
        if (!roles) return []
        try {
            const parsed = JSON.parse(roles)
            return Array.isArray(parsed) ? parsed : [roles]
        } catch {
            return roles.split(',').map((r) => r.trim()).filter(Boolean)
        }
    }

    async function carregar() {
        loading.value = true
        try {
            const { usuarios: lista } = await listarUsuarios(apiFetch)
            usuarios.value = lista
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    // --- Novo usuário ---
    const novoVisible = ref(false)
    const salvando = ref(false)
    const novoForm = reactive({ username: '', senha: '', roles: [] as string[] })

    function abrirNovo() {
        Object.assign(novoForm, { username: '', senha: '', roles: [] })
        novoVisible.value = true
    }

    async function criarUsuario() {
        salvando.value = true
        try {
            await registrarUsuario(apiFetch, {
                username: novoForm.username,
                senha: novoForm.senha,
                roles: novoForm.roles,
            })
            novoVisible.value = false
            notifySuccess('Usuário criado')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvando.value = false
        }
    }

    // --- Desativar ---
    const desativarVisible = ref(false)
    const usuarioDesativar = ref<Usuario | null>(null)

    function confirmarDesativar(u: Usuario) {
        usuarioDesativar.value = u
        desativarVisible.value = true
    }

    async function desativar() {
        if (!usuarioDesativar.value) return
        salvando.value = true
        try {
            await desativarUsuario(apiFetch, usuarioDesativar.value.usuario_id)
            desativarVisible.value = false
            notifyWarn('Desativado', `${usuarioDesativar.value.username} foi desativado.`)
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvando.value = false
        }
    }

    // --- Editar papéis ---
    const edicaoVisible = ref(false)
    const usuarioEdicao = ref<Usuario | null>(null)
    const edicaoRoles = ref<string[]>([])

    function abrirEdicao(u: Usuario) {
        usuarioEdicao.value = u
        edicaoRoles.value = parseRoles(u.roles)
        edicaoVisible.value = true
    }

    async function salvarEdicao() {
        if (!usuarioEdicao.value) return
        salvando.value = true
        try {
            await salvarRolesUsuario(apiFetch, usuarioEdicao.value.usuario_id, edicaoRoles.value)
            edicaoVisible.value = false
            notifySuccess('Papéis atualizados')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvando.value = false
        }
    }

    // --- Reativar ---
    async function reativar(u: Usuario) {
        try {
            await reativarUsuario(apiFetch, u.usuario_id)
            notifySuccess('Reativado', `${u.username} foi reativado.`)
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Alterar senha ---
    const senhaVisible = ref(false)
    const salvandoSenha = ref(false)
    const senhaForm = reactive({ senha_atual: '', nova_senha: '' })

    async function alterarSenha() {
        salvandoSenha.value = true
        try {
            await alterarSenhaModel(apiFetch, senhaForm)
            senhaVisible.value = false
            Object.assign(senhaForm, { senha_atual: '', nova_senha: '' })
            notifySuccess('Senha alterada')
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            salvandoSenha.value = false
        }
    }

    return reactive({
        isAdmin,
        username,
        usuarios,
        loading,
        filters,
        parseRoles,
        carregar,
        rolesOpcoes,
        novoVisible,
        salvando,
        novoForm,
        abrirNovo,
        criarUsuario,
        desativarVisible,
        usuarioDesativar,
        confirmarDesativar,
        desativar,
        edicaoVisible,
        usuarioEdicao,
        edicaoRoles,
        abrirEdicao,
        salvarEdicao,
        reativar,
        senhaVisible,
        salvandoSenha,
        senhaForm,
        alterarSenha,
    })
}
