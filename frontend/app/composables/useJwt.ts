/** Decodifica o payload de um JWT sem validar assinatura (a API já validou). */
export function decodeJwtPayload<T>(token: string): T | null {
  try {
    const payload = token.split('.')[1]
    if (!payload) return null
    const json = atob(payload.replace(/-/g, '+').replace(/_/g, '/'))
    return JSON.parse(decodeURIComponent(escape(json)))
  } catch {
    return null
  }
}
