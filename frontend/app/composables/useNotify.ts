import { toast } from 'vue-sonner'

/** Feedback padronizado via toast — substitui os `toast.add({...})` repetidos em cada página. */
export function useNotify() {
  function notifySuccess(summary: string, detail?: string) {
    toast.success(summary, { description: detail })
  }

  function notifyWarn(summary: string, detail?: string) {
    toast.warning(summary, { description: detail })
  }

  function notifyInfo(summary: string, detail?: string) {
    toast.info(summary, { description: detail })
  }

  function notifyError(message: string, summary = 'Erro') {
    toast.error(summary, { description: message })
  }

  return { notifySuccess, notifyWarn, notifyInfo, notifyError }
}
