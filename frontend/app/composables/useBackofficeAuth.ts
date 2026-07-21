interface BackofficeClaims {
  sub: string
  username: string
  role: string
  permissions: string[]
  exp: number
}

export function useBackofficeAuth() {
  const token = useCookie<string | null>('backoffice_token')
  const claims = computed(() => (token.value ? decodeJwtPayload<BackofficeClaims>(token.value) : null))
  const username = computed(() => claims.value?.username ?? '')
  const role = computed(() => claims.value?.role ?? '')
  const permissions = computed(() => claims.value?.permissions ?? [])
  const isSuperadmin = computed(() => role.value === 'superadmin')

  function hasPermission(perm: string): boolean {
    return isSuperadmin.value || permissions.value.includes(perm)
  }

  async function logout() {
    token.value = null
    await navigateTo('/login')
  }

  return { token, claims, username, role, permissions, isSuperadmin, hasPermission, logout }
}
