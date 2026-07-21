// Aplica a marca whitelabel do subdomínio o quanto antes na inicialização do
// app (inclusive na tela de login, antes da autenticação). No-op para o apex
// (landing) e o backoffice — ver useMarca.
export default defineNuxtPlugin(() => {
    const { carregar } = useMarca()
    carregar()
})
