import { obterConfiguracoes } from '~/models/configuracoes'
import type { BusinessInfo } from '~/composables/useThermalPrint'

/** Dados da empresa (Configurações → Dados da empresa) para uso nas
 * impressões de venda/orçamento. Busca uma vez por sessão de navegação
 * (`useState` — compartilhado entre todos os componentes) e reaproveita. */
export function useEmpresaInfo() {
    const { apiFetch } = useApi()
    const businessInfo = useState<BusinessInfo>('empresa-business-info', () => ({}))
    const carregado = useState('empresa-business-info-carregado', () => false)

    async function garantirCarregado() {
        if (carregado.value) return
        try {
            const cfg = await obterConfiguracoes(apiFetch)
            businessInfo.value = {
                cnpj: cfg.cnpj,
                telefone: cfg.telefone,
                endereco: cfg.endereco,
                chavePix: cfg.chave_pix,
                informacoesAdicionais: cfg.informacoes_adicionais,
            }
            carregado.value = true
        } catch {
            // Impressão não deve falhar por causa disso — segue sem os dados da empresa.
        }
    }

    return { businessInfo, garantirCarregado }
}
