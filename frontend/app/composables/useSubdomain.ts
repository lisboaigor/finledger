const BACKOFFICE_SLUGS = new Set(['admin', 'backoffice'])
// Apelidos do apex: mostram a landing, não são tenants.
const APEX_ALIASES = new Set(['www'])

/**
 * Extrai o subdomínio do host. Com `baseDomain` configurado (produção,
 * NUXT_PUBLIC_BASE_DOMAIN), o apex e hosts fora do domínio-base retornam null
 * — contar labels não funciona para domínios como finledger.com.br, cujo apex
 * já tem 3 partes. Sem `baseDomain` (dev), cai na heurística de labels:
 * demo.localhost → 'demo', localhost/IP → null.
 */
function detectSubdomain(hostname: string, baseDomain: string): string | null {
    if (hostname === 'localhost' || /^\d/.test(hostname)) return null

    if (baseDomain) {
        if (hostname === baseDomain) return null
        if (!hostname.endsWith(`.${baseDomain}`)) return null
        const sub = hostname.slice(0, -(baseDomain.length + 1))
        // Apenas um nível de subdomínio é um tenant válido.
        return sub && !sub.includes('.') ? sub : null
    }

    const parts = hostname.split('.')
    return parts.length >= 2 ? (parts[0] ?? null) : null
}

export function useSubdomain() {
    // useRequestURL resolves the host on both server (request headers) and
    // client (window.location), so subdomain detection also works during SSR.
    const url = useRequestURL()
    const baseDomain = useRuntimeConfig().public.baseDomain
    const subdomain = computed(() => {
        const sub = detectSubdomain(url.hostname, baseDomain)
        return sub && APEX_ALIASES.has(sub) ? null : sub
    })
    const isBackoffice = computed(() => !!subdomain.value && BACKOFFICE_SLUGS.has(subdomain.value))
    const tenantSlug = computed(() => {
        if (!subdomain.value || isBackoffice.value) return null
        return subdomain.value
    })

    return { subdomain, isBackoffice, tenantSlug }
}
