export function useBackofficeApi() {
  const { token } = useBackofficeAuth()
  return createApiClient(token, '/login', 'Ocorreu um erro.')
}
