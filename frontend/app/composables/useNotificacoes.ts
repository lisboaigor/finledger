import type { Component } from 'vue'
import type { BiAlerta } from '~/models/bi'
import {
    Bell,
    Boxes,
    Clock,
    File,
    FileX,
    Percent,
    Phone,
    ShoppingCart,
    Tag,
    TrendingUp,
    UserMinus,
    Wallet,
} from '@lucide/vue'
import { enviarFeedbackAlerta, listarAlertasBi } from '~/models/bi'

const ICONE_ALERTA: Record<string, Component> = {
    A1: Wallet,
    A2: Phone,
    A3: Boxes,
    A4: Boxes,
    A5: File,
    A6: FileX,
    A7: Tag,
    A8: UserMinus,
    A9: ShoppingCart,
    A10: TrendingUp,
    A11: Clock,
    A12: Percent,
}

/** Estado único das notificações do tenant (alertas do motor de BI), dividido
 * entre o sino do topbar e o card "O que fazer hoje" do dashboard — uma carga
 * por sessão, mesma lista, mesmo feedback. Os alertas já chegam ordenados por
 * score (mais importante primeiro). */
export function useNotificacoes() {
    const { apiFetch } = useApi()
    const { notifySuccess, notifyError } = useNotify()

    const alertas = useState<BiAlerta[]>('notificacoes-alertas', () => [])
    const carregado = useState('notificacoes-carregado', () => false)
    const enviandoFeedback = useState<string | null>('notificacoes-enviando', () => null)

    async function carregar(force = false) {
        if (carregado.value && !force) return
        try {
            const { alertas: a } = await listarAlertasBi(apiFetch, 20)
            alertas.value = a
            carregado.value = true
        } catch {
            // BI indisponível — o restante da tela continua.
        }
    }

    async function feedback(alerta: BiAlerta, acao: 'resolvido' | 'ignorado') {
        enviandoFeedback.value = alerta.alerta_id
        try {
            await enviarFeedbackAlerta(apiFetch, alerta.alerta_id, acao)
            alertas.value = alertas.value.filter((a) => a.alerta_id !== alerta.alerta_id)
            notifySuccess(
                acao === 'resolvido' ? 'Marcado como resolvido' : 'Alerta silenciado por 30 dias',
                alerta.titulo,
            )
        } catch {
            notifyError('Não foi possível registrar o feedback')
        } finally {
            enviandoFeedback.value = null
        }
    }

    function iconeAlerta(codigo: string): Component {
        return ICONE_ALERTA[codigo] ?? Bell
    }

    return reactive({ alertas, carregar, feedback, enviandoFeedback, iconeAlerta })
}
