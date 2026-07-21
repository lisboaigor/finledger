/** Assinatura do `apiFetch` retornado por useApi/useBackofficeApi — injetada nos models
 * para que eles fiquem livres de composables do Nuxt (View/ViewModel resolvem o cliente HTTP). */
export type ApiFetch = <T>(path: string, options?: Parameters<typeof $fetch>[1]) => Promise<T>

export interface Opcao {
    label: string
    value: string
}
