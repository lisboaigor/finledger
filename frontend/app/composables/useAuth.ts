interface JwtClaims {
    sub: string
    username: string
    roles: string[]
    tenant_id: string
    tenant_slug: string
    exp: number
}

// Roles defined in the backend (src/auth/mod.rs)
export const ROLES = {
    ADMIN: 'admin',
    VENDEDOR: 'vendedor',
    COMPRADOR: 'comprador',
    ESTOQUISTA: 'estoquista',
    FINANCEIRO: 'financeiro',
    FISCAL: 'fiscal',
} as const

export type RoleName = (typeof ROLES)[keyof typeof ROLES]

export const ROLE_LABELS: Record<RoleName, string> = {
    admin: 'Administrador',
    vendedor: 'Vendedor',
    comprador: 'Comprador',
    estoquista: 'Estoquista',
    financeiro: 'Financeiro',
    fiscal: 'Fiscal',
}

export function useAuth() {
    const token = useCookie<string | null>('auth_token')

    const claims = computed(() => (token.value ? decodeJwtPayload<JwtClaims>(token.value) : null))
    const username = computed(() => claims.value?.username ?? '')
    const roles = computed(() => claims.value?.roles ?? [])
    const isAdmin = computed(() => roles.value.includes(ROLES.ADMIN))
    const tenantId = computed(() => claims.value?.tenant_id ?? '')
    const tenantSlug = computed(() => claims.value?.tenant_slug ?? '')

    // Admin bypasses all role checks
    function hasRole(...required: string[]): boolean {
        if (isAdmin.value) return true
        return required.some((r) => roles.value.includes(r))
    }

    const canVender = computed(() => hasRole(ROLES.VENDEDOR))
    const canComprar = computed(() => hasRole(ROLES.COMPRADOR))
    const canEstoque = computed(() => hasRole(ROLES.ESTOQUISTA))
    const canFinanceiro = computed(() => hasRole(ROLES.FINANCEIRO))
    const canFiscal = computed(() => hasRole(ROLES.FISCAL))

    async function logout() {
        token.value = null
        await navigateTo('/login')
    }

    return {
        token,
        claims,
        username,
        roles,
        isAdmin,
        hasRole,
        canVender,
        canComprar,
        canEstoque,
        canFinanceiro,
        canFiscal,
        logout,
        tenantId,
        tenantSlug,
    }
}
