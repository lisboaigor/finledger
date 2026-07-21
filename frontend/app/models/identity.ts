import type { ApiFetch } from './shared'

export interface Usuario {
    usuario_id: string
    username: string
    roles: string
    ativo: boolean
}

export interface NovoUsuarioPayload {
    username: string
    senha: string
    roles: string[]
}

export interface AlterarSenhaPayload {
    senha_atual: string
    nova_senha: string
}

export interface TenantLoginPayload {
    slug: string
    username: string
    senha: string
}

export interface BackofficeLoginPayload {
    username: string
    senha: string
}

export function listarUsuarios(apiFetch: ApiFetch) {
    return apiFetch<{ usuarios: Usuario[] }>('/auth/usuarios')
}

export function registrarUsuario(apiFetch: ApiFetch, payload: NovoUsuarioPayload) {
    return apiFetch('/auth/registrar', { method: 'POST', body: payload })
}

export function desativarUsuario(apiFetch: ApiFetch, usuarioId: string) {
    return apiFetch(`/auth/usuarios/${usuarioId}/desativar`, { method: 'POST' })
}

export function reativarUsuario(apiFetch: ApiFetch, usuarioId: string) {
    return apiFetch(`/auth/usuarios/${usuarioId}/reativar`, { method: 'POST' })
}

export function salvarRolesUsuario(apiFetch: ApiFetch, usuarioId: string, roles: string[]) {
    return apiFetch(`/auth/usuarios/${usuarioId}`, { method: 'PUT', body: { roles } })
}

export function alterarSenha(apiFetch: ApiFetch, payload: AlterarSenhaPayload) {
    return apiFetch('/auth/alterar-senha', { method: 'POST', body: payload })
}

export function loginTenant(apiFetch: ApiFetch, payload: TenantLoginPayload) {
    return apiFetch<{ token: string }>('/auth/login', { method: 'POST', body: payload })
}

export function loginBackoffice(apiFetch: ApiFetch, payload: BackofficeLoginPayload) {
    return apiFetch<{ token: string }>('/backoffice/auth/login', { method: 'POST', body: payload })
}
