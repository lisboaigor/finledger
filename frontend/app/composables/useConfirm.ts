import { reactive } from 'vue'

export interface ConfirmOptions {
  header?: string
  message: string
  acceptLabel?: string
  rejectLabel?: string
  /** Estilo do botão de confirmação — 'danger' pinta em vermelho (exclusões). */
  variant?: 'default' | 'danger'
  accept: () => void | Promise<void>
  reject?: () => void
}

const state = reactive({
  visible: false,
  options: null as ConfirmOptions | null,
})

/** Confirmação global via AlertDialog (single instance montada no layout). */
export function useConfirm() {
  function require(options: ConfirmOptions) {
    state.options = options
    state.visible = true
  }
  return { require }
}

/** Usado apenas pelo componente <ConfirmDialog />. */
export function useConfirmState() {
  return state
}
