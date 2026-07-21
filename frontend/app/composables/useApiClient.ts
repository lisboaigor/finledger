/** Cria um cliente HTTP autenticado; compartilhado por useApi e useBackofficeApi,
 * que só variam no cookie de token, na rota de login e na mensagem de fallback. */
export function createApiClient(token: Ref<string | null>, loginPath: string, fallbackMessage: string) {
  const config = useRuntimeConfig()

  function apiFetch<T>(path: string, options: Parameters<typeof $fetch>[1] = {}) {
    return $fetch<T>(`${config.public.apiBase}${path}`, {
      ...options,
      headers: {
        ...(options.headers as Record<string, string> | undefined),
        ...(token.value ? { Authorization: `Bearer ${token.value}` } : {}),
      },
      onResponseError({ response }) {
        if (response.status === 401) {
          token.value = null
          if (import.meta.client) navigateTo(loginPath)
        }
      },
    })
  }

  /** Extrai a mensagem de erro retornada pela API ({ error: "..." }). */
  function apiErrorMessage(e: unknown, fallback = fallbackMessage): string {
    const data = (e as { data?: { error?: string } })?.data
    return data?.error ?? fallback
  }

  return { apiFetch, apiErrorMessage }
}
