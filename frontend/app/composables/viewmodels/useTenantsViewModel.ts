import type { Tenant } from '~/models/backoffice'
import {
    alterarPlanoTenant,
    criarTenant as criarTenantModel,
    editarTenant,
    impersonarTenant,
    listarTenants,
    reativarTenant,
    suspenderTenant,
} from '~/models/backoffice'

const planosOpcoes = [
    { label: 'Básico', value: 'basico' },
    { label: 'Profissional', value: 'profissional' },
    { label: 'Enterprise', value: 'enterprise' },
]

/** ViewModel da página de Tenants (backoffice): concentra estado e regras de negócio;
 * a View só lê estado e dispara ações. */
export function useTenantsViewModel() {
    const { apiFetch, apiErrorMessage } = useBackofficeApi()
    const { hasPermission, token } = useBackofficeAuth()
    const { notifySuccess, notifyError } = useNotify()

    function verificarAcesso() {
        if (!token.value) navigateTo('/login')
    }

    const tenants = ref<Tenant[]>([])
    const loading = ref(false)
    const filters = ref({ global: { value: null as string | null, matchMode: 'contains' } })

    const canRead = computed(() => hasPermission('tenants:read'))

    async function carregar() {
        // Without tenants:read the request would just 403 — show the
        // permission notice in the page instead of an error toast.
        if (!canRead.value) return
        loading.value = true
        try {
            const { tenants: t } = await listarTenants(apiFetch)
            tenants.value = t
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            loading.value = false
        }
    }

    async function suspender(id: string) {
        try {
            await suspenderTenant(apiFetch, id)
            notifySuccess('Suspenso')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    async function reativar(id: string) {
        try {
            await reativarTenant(apiFetch, id)
            notifySuccess('Reativado')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    // --- Novo tenant ---
    const novoVisible = ref(false)
    const saving = ref(false)
    const novoForm = reactive({ slug: '', nome: '', admin_username: 'admin', admin_senha: '' })

    async function criarTenant() {
        saving.value = true
        try {
            await criarTenantModel(apiFetch, novoForm)
            novoVisible.value = false
            Object.assign(novoForm, { slug: '', nome: '', admin_username: 'admin', admin_senha: '' })
            notifySuccess('Tenant criado')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            saving.value = false
        }
    }

    // --- Editar tenant ---
    const edicaoVisible = ref(false)
    const edicaoForm = reactive({ tenant_id: '', nome: '' })

    function abrirEdicao(tenant: Tenant) {
        edicaoForm.tenant_id = tenant.tenant_id
        edicaoForm.nome = tenant.nome
        edicaoVisible.value = true
    }

    async function salvarEdicao() {
        saving.value = true
        try {
            await editarTenant(apiFetch, edicaoForm.tenant_id, edicaoForm.nome)
            edicaoVisible.value = false
            notifySuccess('Tenant atualizado')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            saving.value = false
        }
    }

    // --- Alterar plano ---
    const planoVisible = ref(false)
    const planoForm = reactive({ tenant_id: '', plano: 'basico' })

    function abrirPlano(tenant: Tenant) {
        planoForm.tenant_id = tenant.tenant_id
        planoForm.plano = tenant.plano
        planoVisible.value = true
    }

    async function salvarPlano() {
        saving.value = true
        try {
            await alterarPlanoTenant(apiFetch, planoForm.tenant_id, planoForm.plano)
            planoVisible.value = false
            notifySuccess('Plano atualizado')
            await carregar()
        } catch (e) {
            notifyError(apiErrorMessage(e))
        } finally {
            saving.value = false
        }
    }

    // --- Impersonation ---
    const impersonarVisible = ref(false)
    const impersonarToken = ref('')
    const impersonarUrl = ref('')

    /** Builds the tenant login URL that consumes the support token directly:
     * swaps the current subdomain (admin/backoffice) for the tenant slug and
     * appends the token as a query param handled by /login. */
    function impersonationUrl(slug: string, token: string): string {
        const { protocol, host } = window.location
        const baseHost = host.split('.').slice(1).join('.')
        return `${protocol}//${slug}.${baseHost}/login?token=${encodeURIComponent(token)}`
    }

    async function impersonar(tenant: Tenant) {
        try {
            const { token: t } = await impersonarTenant(apiFetch, tenant.tenant_id)
            impersonarToken.value = t
            impersonarUrl.value = impersonationUrl(tenant.slug, t)
            impersonarVisible.value = true
        } catch (e) {
            notifyError(apiErrorMessage(e))
        }
    }

    function abrirImpersonacao() {
        window.open(impersonarUrl.value, '_blank', 'noopener')
    }

    async function copiarToken() {
        await navigator.clipboard.writeText(impersonarUrl.value)
        notifySuccess('Copiado!', undefined, 2000)
    }

    return reactive({
        hasPermission,
        canRead,
        verificarAcesso,
        tenants,
        loading,
        filters,
        carregar,
        suspender,
        reativar,
        novoVisible,
        saving,
        novoForm,
        criarTenant,
        edicaoVisible,
        edicaoForm,
        abrirEdicao,
        salvarEdicao,
        planoVisible,
        planoForm,
        planosOpcoes,
        abrirPlano,
        salvarPlano,
        impersonarVisible,
        impersonarToken,
        impersonarUrl,
        impersonar,
        abrirImpersonacao,
        copiarToken,
    })
}
