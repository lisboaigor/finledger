import type { ApiFetch } from './shared'

export interface Admin {
    user_id: string
    username: string
    role: string
    permissions: string[]
    ativo: boolean
}

export interface NovoAdminPayload {
    username: string
    senha: string
    permissions: string[]
}

export interface Tenant {
    tenant_id: string
    slug: string
    nome: string
    status: string
    plano: string
}

export interface NovoTenantPayload {
    slug: string
    nome: string
    admin_username: string
    admin_senha: string
}

export interface TenantRevenue {
    tenant_id: string
    slug: string
    nome: string
    plano: string
    status: string
    total_cents: number
    sales_count: number
    last_30d_cents: number
    last_30d_count: number
    prev_30d_cents: number
    avg_ticket_cents: number
}

export interface MonthlyRevenue {
    month: string
    total_cents: number
    sales_count: number
}

export interface TenantMonthlyRevenue {
    tenant_id: string
    month: string
    total_cents: number
}

export interface DailyRevenue {
    day: string
    total_cents: number
    sales_count: number
}

export interface PlatformStats {
    total_users: number
    active_users: number
    total_products: number
    total_clients: number
    today_cents: number
    today_count: number
}

export interface RevenueOverview {
    stats: PlatformStats
    tenants: TenantRevenue[]
    monthly: MonthlyRevenue[]
    monthly_by_tenant: TenantMonthlyRevenue[]
    daily: DailyRevenue[]
}

export function getRevenueOverview(apiFetch: ApiFetch) {
    return apiFetch<RevenueOverview>('/backoffice/revenue')
}

// --- Admins de suporte ---

export function listarAdmins(apiFetch: ApiFetch) {
    return apiFetch<{ admins: Admin[] }>('/backoffice/admins')
}

export function desativarAdmin(apiFetch: ApiFetch, id: string) {
    return apiFetch(`/backoffice/admins/${id}/desativar`, { method: 'POST' })
}

export function reativarAdmin(apiFetch: ApiFetch, id: string) {
    return apiFetch(`/backoffice/admins/${id}/reativar`, { method: 'POST' })
}

export function criarAdmin(apiFetch: ApiFetch, payload: NovoAdminPayload) {
    return apiFetch('/backoffice/admins', { method: 'POST', body: payload })
}

export function changeAdminPassword(apiFetch: ApiFetch, userId: string, password: string) {
    return apiFetch(`/backoffice/admins/${userId}/password`, {
        method: 'POST',
        body: { password },
    })
}

export function salvarPermissoesAdmin(apiFetch: ApiFetch, userId: string, permissions: string[]) {
    return apiFetch(`/backoffice/admins/${userId}/permissoes`, {
        method: 'POST',
        body: { permissions },
    })
}

// --- Tenants ---

export function listarTenants(apiFetch: ApiFetch) {
    return apiFetch<{ tenants: Tenant[] }>('/backoffice/tenants')
}

export function suspenderTenant(apiFetch: ApiFetch, id: string) {
    return apiFetch(`/backoffice/tenants/${id}/suspender`, { method: 'POST' })
}

export function reativarTenant(apiFetch: ApiFetch, id: string) {
    return apiFetch(`/backoffice/tenants/${id}/reativar`, { method: 'POST' })
}

export function criarTenant(apiFetch: ApiFetch, payload: NovoTenantPayload) {
    return apiFetch('/backoffice/tenants', { method: 'POST', body: payload })
}

export function editarTenant(apiFetch: ApiFetch, tenantId: string, nome: string) {
    return apiFetch(`/backoffice/tenants/${tenantId}`, { method: 'PUT', body: { nome } })
}

export function alterarPlanoTenant(apiFetch: ApiFetch, tenantId: string, plano: string) {
    return apiFetch(`/backoffice/tenants/${tenantId}/plano`, { method: 'POST', body: { plano } })
}

export function impersonarTenant(apiFetch: ApiFetch, tenantId: string) {
    return apiFetch<{ token: string }>(`/backoffice/tenants/${tenantId}/impersonar`, { method: 'POST' })
}
