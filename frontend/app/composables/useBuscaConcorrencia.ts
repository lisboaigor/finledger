/** Atalho de pesquisa de preços da concorrência: abre o Google Shopping numa
 * aba nova com o produto já preenchido. Puramente client-side — scraping
 * automático de sites de concorrentes está fora de escopo (frágil e sem API
 * estável); o registro do preço visto é manual, na tela de Catálogo. */
export function useBuscaConcorrencia() {
    function abrirPesquisaGoogle(sku: string, descricao: string) {
        const q = encodeURIComponent(`${descricao} ${sku}`.trim())
        window.open(`https://www.google.com/search?tbm=shop&q=${q}`, '_blank', 'noopener')
    }

    return { abrirPesquisaGoogle }
}
