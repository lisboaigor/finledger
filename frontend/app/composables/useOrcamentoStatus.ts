export function useOrcamentoStatus() {
    function statusSeverity(status: string) {
        return (
            {
                Aceito: 'success',
                Emitido: 'info',
                Rascunho: 'secondary',
                Recusado: 'danger',
                Expirado: 'warn',
                Cancelado: 'danger',
                ConvertidoEmVenda: 'success',
            }[status] ?? 'secondary'
        )
    }

    return { statusSeverity }
}
