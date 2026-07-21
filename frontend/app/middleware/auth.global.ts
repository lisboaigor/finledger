// Rotas exclusivas do backoffice (acessíveis sem prefixo via subdomínio backoffice.*)
const BACKOFFICE_ROUTES = ['/dashboard', '/tenants', '/admins']
// Rotas públicas de cada contexto
const PUBLIC_ROUTES = ['/login']

export default defineNuxtRouteMiddleware((to) => {
    const { isBackoffice, tenantSlug } = useSubdomain()

    // Host sem subdomínio (apex finledger.com.br, localhost puro, IP) não resolve
    // tenant nem backoffice: só existe a landing na raiz — o resto volta pra ela.
    if (!isBackoffice.value && !tenantSlug.value) {
        if (to.path === '/') return
        return navigateTo('/')
    }

    // ── Subdomínio backoffice (ex.: admin.finledger.com.br) ──────────────────
    if (isBackoffice.value) {
        if (PUBLIC_ROUTES.includes(to.path)) return

        const { token } = useBackofficeAuth()
        if (!token.value) return navigateTo('/login')

        // Permite apenas rotas do backoffice; qualquer outra vai para a visão geral
        if (!BACKOFFICE_ROUTES.some((r) => to.path.startsWith(r))) {
            return navigateTo('/dashboard')
        }
        return
    }

    // ── Subdomínio tenant (ex.: empresa.finledger.com.br) ────────────────────
    // Bloqueia acesso a rotas de backoffice no contexto de tenant
    if (tenantSlug.value) {
        if (BACKOFFICE_ROUTES.some((r) => to.path.startsWith(r))) {
            return navigateTo('/')
        }
    }

    if (PUBLIC_ROUTES.includes(to.path)) return

    // ── Auth de tenant ──────────────────────────────────────────────────────
    const { token, tenantId } = useAuth()

    if (!token.value || !tenantId.value) {
        const redirect = to.fullPath !== '/' ? `?redirect=${encodeURIComponent(to.fullPath)}` : ''
        return navigateTo(`/login${redirect}`)
    }

    // Valida que o token pertence ao mesmo tenant que o subdomínio
    if (tenantSlug.value && import.meta.client) {
        const { tenantSlug: tokenSlug } = useAuth()
        if (tokenSlug.value && tokenSlug.value !== tenantSlug.value) {
            const cookie = useCookie('auth_token')
            cookie.value = null
            return navigateTo('/login')
        }
    }
})
