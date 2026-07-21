/** Busca aproximada client-side: ignora acento, aceita palavras em qualquer
 * ordem/incompletas e tolera erro de digitação — usada em todas as telas com
 * busca (listas já carregadas no navegador, sem round-trip ao backend). */

function normalizar(s: string): string {
    return s
        .normalize('NFD')
        .replace(/[̀-ͯ]/g, '')
        .toLowerCase()
        .trim()
}

function distancia(a: string, b: string): number {
    const m = a.length
    const n = b.length
    if (m === 0) return n
    if (n === 0) return m

    let prev = Array.from({ length: n + 1 }, (_, j) => j)
    let curr = new Array<number>(n + 1)

    for (let i = 1; i <= m; i++) {
        curr[0] = i
        for (let j = 1; j <= n; j++) {
            const custo = a[i - 1] === b[j - 1] ? 0 : 1
            curr[j] = Math.min(
                (prev[j] ?? 0) + 1,
                (curr[j - 1] ?? 0) + 1,
                (prev[j - 1] ?? 0) + custo,
            )
        }
        ;[prev, curr] = [curr, prev]
    }
    return prev[n] ?? 0
}

/** Erros tolerados proporcionalmente ao tamanho da palavra digitada. */
function limiar(tamanho: number): number {
    if (tamanho <= 3) return 0
    if (tamanho <= 6) return 1
    return 2
}

/** Palavra digitada casa com um token do item por substring (cobre digitação
 * parcial: "fre" acha "freio") ou por distância de edição dentro do limiar
 * (cobre erro de digitação: "pastlha" acha "pastilha"). */
function palavraCasa(palavra: string, tokens: string[]): boolean {
    return tokens.some((tok) => tok.includes(palavra) || distancia(palavra, tok) <= limiar(palavra.length))
}

/** Filtra `itens` pela `consulta` digitada: toda palavra da consulta precisa
 * casar com algum token do texto do item (AND entre palavras, ordem livre). */
export function buscarAproximado<T>(itens: T[], consulta: string, textoDoItem: (item: T) => string): T[] {
    const q = normalizar(consulta)
    if (!q) return itens

    const palavras = q.split(/\s+/).filter(Boolean)
    return itens.filter((item) => {
        const tokens = normalizar(textoDoItem(item)).split(/\s+/).filter(Boolean)
        return palavras.every((p) => palavraCasa(p, tokens))
    })
}

export function useFuzzySearch() {
    return { buscarAproximado, normalizar }
}
