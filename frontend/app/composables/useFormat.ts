const brl = new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' })

export function useFormat() {
  /** Formata um valor em centavos (i64) como moeda BRL. */
  function formatCentavos(centavos: number | null | undefined): string {
    if (centavos == null) return '—'
    return brl.format(centavos / 100)
  }

  /** Formata um valor em reais (number) como moeda BRL. */
  function formatBRL(valor: number | null | undefined): string {
    if (valor == null) return '—'
    return brl.format(valor)
  }

  /** Converte reais (number) para centavos (inteiro). */
  function toCentavos(reais: number | null | undefined): number {
    if (reais == null) return 0
    return Math.round(reais * 100)
  }

  function formatNumber(value: number | null | undefined): string {
    if (value == null) return '—'
    return new Intl.NumberFormat('pt-BR').format(value)
  }

  return { formatCentavos, formatBRL, toCentavos, formatNumber }
}
