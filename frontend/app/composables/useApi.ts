export function useApi() {
  const token = useCookie<string | null>('auth_token')
  const { apiFetch, apiErrorMessage } = createApiClient(
    token,
    '/login',
    'Ocorreu um erro. Tente novamente.',
  )

  return { apiFetch, apiErrorMessage, token }
}
