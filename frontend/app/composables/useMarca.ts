import type { Marca } from '~/models/marca'
import { familiaDaFonte, MARCA_VAZIA } from '~/models/marca'

/** CSS custom properties que o whitelabel pode sobrescrever. Mantida explícita
 * para limpar a personalização de uma vez (voltar ao padrão) ao pré-visualizar
 * ou restaurar, sem deixar sobra. */
const VARS_GERENCIADAS = [
    '--primary', '--primary-foreground', '--ring', '--chart-1',
    '--sidebar-primary', '--sidebar-primary-foreground', '--sidebar-ring',
    '--marca-fonte', '--marca-fonte-escala', '--marca-fonte-cor',
] as const

function hexParaRgb(hex: string): [number, number, number] | null {
    const m = /^#([0-9a-f]{6})$/i.exec(hex.trim())
    if (!m) return null
    const n = Number.parseInt(m[1]!, 16)
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}

/** Luminância relativa (sRGB) para decidir texto claro vs. escuro sobre a cor. */
function luminancia(hex: string): number {
    const rgb = hexParaRgb(hex)
    if (!rgb) return 1
    const [r, g, b] = rgb.map((c) => {
        const s = c / 255
        return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
    }) as [number, number, number]
    return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

/** Cor de texto com contraste suficiente sobre `hex` (quase-preto ou quase-branco). */
function textoContraste(hex: string): string {
    return luminancia(hex) > 0.5 ? '#0a0a0a' : '#fafafa'
}

function montarOverrides(marca: Marca): Record<string, string> {
    const vars: Record<string, string> = {}

    const primaria = marca.marca_cor_primaria
    if (primaria && hexParaRgb(primaria)) {
        const fgP = textoContraste(primaria)
        Object.assign(vars, {
            '--primary': primaria,
            '--primary-foreground': fgP,
            '--ring': primaria,
            '--chart-1': primaria,
            '--sidebar-primary': primaria,
            '--sidebar-primary-foreground': fgP,
            '--sidebar-ring': primaria,
        })
    }

    const familia = familiaDaFonte(marca.marca_fonte)
    if (familia) vars['--marca-fonte'] = familia

    // Tamanho: percentual (50–200) → fator de escala aplicado ao base de cada
    // local pelo `.brand-wordmark`. Fora da faixa é ignorado (cai no padrão).
    const tam = marca.marca_fonte_tamanho
    if (typeof tam === 'number' && tam >= 50 && tam <= 200) {
        vars['--marca-fonte-escala'] = String(tam / 100)
    }

    const corFonte = marca.marca_fonte_cor
    if (corFonte && hexParaRgb(corFonte)) vars['--marca-fonte-cor'] = corFonte

    return vars
}

// Estado único por sessão de navegação (compartilhado entre todos os
// componentes): a marca já resolvida do subdomínio atual.
export function useMarca() {
    const marca = useState<Marca>('marca', () => ({ ...MARCA_VAZIA }))
    const carregada = useState('marca-carregada', () => false)
    const { tenantSlug, isBackoffice } = useSubdomain()

    /** Aplica (ou limpa) os overrides no <html>. Só no cliente. */
    function aplicar(m: Marca) {
        if (!import.meta.client) return
        const root = document.documentElement
        for (const v of VARS_GERENCIADAS) root.style.removeProperty(v)
        for (const [k, val] of Object.entries(montarOverrides(m))) {
            root.style.setProperty(k, val)
        }
    }

    /** Busca a marca do subdomínio e aplica. Backoffice e apex (landing) não
     * têm marca de tenant — no-op. Idempotente por sessão. */
    async function carregar(force = false) {
        if (carregada.value && !force) return
        if (isBackoffice.value || !tenantSlug.value) {
            carregada.value = true
            return
        }
        try {
            const config = useRuntimeConfig()
            const m = await $fetch<Marca>(`${config.public.apiBase}/tenants/${tenantSlug.value}/marca`)
            marca.value = { ...MARCA_VAZIA, ...m }
            aplicar(marca.value)
            carregada.value = true
        } catch {
            // Sem marca personalizada disponível — segue no tema padrão.
            carregada.value = true
        }
    }

    /** Pré-visualização ao vivo (tela de configuração), sem persistir. */
    function previsualizar(m: Marca) {
        aplicar(m)
    }

    /** Reaplica a marca salva (descarta uma pré-visualização em andamento). */
    function restaurar() {
        aplicar(marca.value)
    }

    const nome = computed(() => marca.value.marca_nome || 'Finledger')
    const logoDataUri = computed(() => marca.value.marca_logo_data_uri || null)

    return { marca, nome, logoDataUri, carregar, aplicar, previsualizar, restaurar }
}
