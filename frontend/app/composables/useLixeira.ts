/** Estado genérico de uma lixeira (vendas, orçamentos): itens arquivados pela
 * rotina de limpeza, com restauração pelo gestor. Nada aqui exclui dado — a
 * lixeira só devolve visibilidade. */
export function useLixeira<T>(opts: {
    listar: () => Promise<T[]>
    restaurar: (id: string) => Promise<unknown>
    idDe: (item: T) => string
    /** Recarrega a listagem principal após uma restauração. */
    aposRestaurar?: () => void | Promise<void>
}) {
    const { notifySuccess, notifyError, notifyInfo } = useNotify()

    const visible = ref(false)
    const loading = ref(false)
    const itens = ref<T[]>([]) as Ref<T[]>
    const restaurando = ref<string | null>(null)

    async function abrir() {
        visible.value = true
        loading.value = true
        try {
            itens.value = await opts.listar()
            if (!itens.value.length) notifyInfo('Lixeira vazia', 'Nada foi arquivado até agora.')
        } catch {
            notifyError('Não foi possível carregar a lixeira.')
        } finally {
            loading.value = false
        }
    }

    async function restaurar(item: T) {
        const id = opts.idDe(item)
        restaurando.value = id
        try {
            await opts.restaurar(id)
            itens.value = itens.value.filter((i) => opts.idDe(i) !== id)
            notifySuccess('Restaurado', 'O registro voltou para a listagem.')
            await opts.aposRestaurar?.()
        } catch {
            notifyError('Não foi possível restaurar.')
        } finally {
            restaurando.value = null
        }
    }

    return reactive({ visible, loading, itens, restaurando, abrir, restaurar })
}
