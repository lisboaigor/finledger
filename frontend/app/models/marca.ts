import type { ApiFetch } from './shared'

/** Identidade visual whitelabel do tenant. Todos os campos opcionais — nulos
 * caem no tema/marca padrão (Finledger). O logo trafega como data URI. */
export interface Marca {
    marca_nome: string | null
    marca_logo_data_uri: string | null
    marca_cor_primaria: string | null
    /** Chave da fonte do wordmark (ver FONTES_MARCA). Nula → Grand Hotel. */
    marca_fonte: string | null
    /** Tamanho do wordmark em % (50–200) sobre o base de cada local. Nulo → 100. */
    marca_fonte_tamanho: number | null
    /** Cor do texto do wordmark, hex #RRGGBB. Nula → cor herdada. */
    marca_fonte_cor: string | null
}

export const MARCA_VAZIA: Marca = {
    marca_nome: null,
    marca_logo_data_uri: null,
    marca_cor_primaria: null,
    marca_fonte: null,
    marca_fonte_tamanho: null,
    marca_fonte_cor: null,
}

/** Faixa de tamanho do wordmark, em pontos percentuais. */
export const TAMANHO_MARCA_MIN = 50
export const TAMANHO_MARCA_MAX = 200
export const TAMANHO_MARCA_PADRAO = 100

/** Conjunto curado de fontes para o nome da marca. A chave é o que persiste; o
 * `familia` alimenta a CSS custom property `--marca-fonte`. Todas carregadas no
 * head (nuxt.config) — o navegador só baixa o arquivo da que for de fato usada.
 * A ausência de escolha (null) usa a fonte padrão Grand Hotel. */
export interface FonteMarca {
    chave: string
    rotulo: string
    familia: string
}

export const FONTES_MARCA: FonteMarca[] = [
    { chave: 'grand-hotel', rotulo: 'Grand Hotel (manuscrita)', familia: "'Grand Hotel', cursive" },
    { chave: 'pacifico', rotulo: 'Pacifico (manuscrita)', familia: "'Pacifico', cursive" },
    { chave: 'playfair', rotulo: 'Playfair Display (serifada)', familia: "'Playfair Display', serif" },
    { chave: 'poppins', rotulo: 'Poppins (sem serifa)', familia: "'Poppins', sans-serif" },
    { chave: 'montserrat', rotulo: 'Montserrat (sem serifa)', familia: "'Montserrat', sans-serif" },
    { chave: 'bricolage', rotulo: 'Bricolage Grotesque (display)', familia: "'Bricolage Grotesque', sans-serif" },
    { chave: 'inter', rotulo: 'Inter (igual à interface)', familia: "'Inter', sans-serif" },
]

/** font-family da chave escolhida (ou null se desconhecida/ausente). */
export function familiaDaFonte(chave: string | null): string | null {
    if (!chave) return null
    return FONTES_MARCA.find((f) => f.chave === chave)?.familia ?? null
}

/** Grava a marca do tenant atual (admin). */
export function atualizarMarca(apiFetch: ApiFetch, marca: Marca) {
    return apiFetch('/configuracoes/marca', { method: 'PUT', body: marca })
}
