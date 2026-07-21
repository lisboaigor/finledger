import type { ApiFetch } from './shared'

export interface Cliente {
    cliente_id: string
    nome: string
    cpf_cnpj: string
    telefone: string | null
    email: string | null
    bloqueado: boolean
    ativo: boolean
}

export interface ClienteDados {
    nome: string
    cpf_cnpj: string
    telefone: string | null
    email: string | null
}

export function listarClientes(apiFetch: ApiFetch) {
    return apiFetch<{ clientes: Cliente[] }>('/crm/clientes')
}

export function criarCliente(apiFetch: ApiFetch, dados: ClienteDados) {
    return apiFetch('/crm/clientes', { method: 'POST', body: dados })
}

export function atualizarCliente(apiFetch: ApiFetch, clienteId: string, dados: Omit<ClienteDados, 'cpf_cnpj'>) {
    return apiFetch(`/crm/clientes/${clienteId}`, { method: 'PUT', body: dados })
}

export function bloquearCliente(apiFetch: ApiFetch, clienteId: string, motivo: string) {
    return apiFetch(`/crm/clientes/${clienteId}/bloquear`, { method: 'POST', body: { motivo } })
}

export function desbloquearCliente(apiFetch: ApiFetch, clienteId: string) {
    return apiFetch(`/crm/clientes/${clienteId}/desbloquear`, { method: 'POST' })
}

export function alternarAtivoCliente(apiFetch: ApiFetch, clienteId: string, ativo: boolean) {
    return apiFetch(`/crm/clientes/${clienteId}/${ativo ? 'desativar' : 'reativar'}`, { method: 'POST' })
}
