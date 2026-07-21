import { loginBackoffice, loginTenant } from '~/models/identity'

/** ViewModel da página de Login: resolve o fluxo (tenant vs. backoffice) via useSubdomain
 * e concentra estado/regra de negócio dos dois formulários. */
export function useLoginViewModel() {
    const route = useRoute()
    const config = useRuntimeConfig()
    const { isBackoffice, tenantSlug } = useSubdomain()

    const loading = ref(false)
    const error = ref('')

    // --- Backoffice ---
    const { token: boToken } = useBackofficeAuth()
    const boForm = reactive({ username: '', senha: '' })

    // Fetch "cru" (sem Authorization/interceptor de 401) igual ao comportamento original,
    // já que aqui ainda não há token e um 401 deve só exibir o erro, não redirecionar.
    function rawFetch<T>(path: string, options?: Parameters<typeof $fetch>[1]) {
        return $fetch<T>(`${config.public.apiBase}${path}`, options)
    }

    async function handleBackofficeLogin() {
        loading.value = true
        error.value = ''
        try {
            const { token: jwt } = await loginBackoffice(rawFetch, boForm)
            boToken.value = jwt
            await navigateTo('/dashboard')
        } catch {
            error.value = 'Usuário ou senha inválidos.'
        } finally {
            loading.value = false
        }
    }

    // --- Tenant ---
    const { apiFetch, token: tenantToken } = useApi()
    const tenantForm = reactive({ username: '', senha: '' })

    async function handleTenantLogin() {
        if (!tenantSlug.value) return
        loading.value = true
        error.value = ''
        try {
            const { token: jwt } = await loginTenant(apiFetch, {
                slug: tenantSlug.value,
                username: tenantForm.username,
                senha: tenantForm.senha,
            })
            tenantToken.value = jwt
            const redirect = (route.query.redirect as string) ?? '/'
            await navigateTo(decodeURIComponent(redirect))
        } catch {
            error.value = 'Usuário ou senha inválidos.'
        } finally {
            loading.value = false
        }
    }

    function verificarSessao() {
        // Direct token hand-off (backoffice impersonation): /login?token=<jwt>
        // on a tenant subdomain logs in with the support token issued by the
        // backoffice. The global middleware still validates the slug claim.
        const directToken = route.query.token
        if (!isBackoffice.value && typeof directToken === 'string' && directToken) {
            tenantToken.value = directToken
            navigateTo('/')
            return
        }

        if (isBackoffice.value && boToken.value) navigateTo('/dashboard')
        else if (!isBackoffice.value && tenantToken.value) navigateTo('/')
    }

    return reactive({
        isBackoffice,
        tenantSlug,
        loading,
        error,
        boForm,
        handleBackofficeLogin,
        tenantForm,
        handleTenantLogin,
        verificarSessao,
    })
}
