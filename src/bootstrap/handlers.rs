use std::sync::Arc;

use pharos_app::EventBus;
use pharos_postgres::Pool;

use super::repositories::Repositories;
use crate::auth::AuthConfig;
use crate::backoffice::handlers::BackofficeHandlers;
use crate::bi::application::handler::BiHandlers;
use crate::catalogo::application::handler::CatalogoHandlers;
use crate::catalogo::infrastructure::precificacao_repository::PostgresPrecificacaoRepository;
use crate::compras::application::handler::ComprasHandlers;
use crate::crm::application::handler::CrmHandlers;
use crate::estoque::application::handler::EstoqueHandlers;
use crate::financeiro::application::handler::FinanceiroHandlers;
use crate::fiscal::application::handler::FiscalHandlers;
use crate::fiscal::infrastructure::aliquotas::PostgresAliquotaProvider;
use crate::fiscal::infrastructure::sefaz::StubSefazClient;
use crate::fornecedores::application::handler::FornecedoresHandlers;
use crate::identity::application::handler::IdentityHandlers;
use crate::orcamentos::application::handler::OrcamentosHandlers;
use crate::tenants::repository::TenantRepository;
use crate::vendas::application::handler::VendasHandlers;

/// Declara `Handlers` (usado durante o bootstrap) e `AppState` (usado pelos
/// handlers HTTP em `src/web`) a partir de uma única lista de campos, e gera
/// `Handlers::into_state`. Isso evita que as duas structs e o mapeamento entre
/// elas divirjam silenciosamente — antes eram três listas mantidas à mão.
macro_rules! define_handlers_and_state {
    ($($field:ident : $ty:ty),+ $(,)?) => {
        pub struct Handlers {
            $(pub $field: $ty,)+
        }

        #[derive(Clone)]
        pub struct AppState {
            pub pool: Pool,
            $(pub $field: $ty,)+
        }

        impl Handlers {
            pub fn into_state(self, pool: Pool) -> AppState {
                AppState {
                    pool,
                    $($field: self.$field,)+
                }
            }
        }
    };
}

define_handlers_and_state! {
    auth: Arc<AuthConfig>,
    backoffice: Arc<BackofficeHandlers>,
    tenants: Arc<TenantRepository>,
    catalogo: Arc<CatalogoHandlers>,
    precificacao: Arc<PostgresPrecificacaoRepository>,
    crm: Arc<CrmHandlers>,
    estoque: Arc<EstoqueHandlers>,
    vendas: Arc<VendasHandlers>,
    fornecedores: Arc<FornecedoresHandlers>,
    orcamentos: Arc<OrcamentosHandlers>,
    compras: Arc<ComprasHandlers>,
    financeiro: Arc<FinanceiroHandlers>,
    fiscal: Arc<FiscalHandlers<StubSefazClient, PostgresAliquotaProvider>>,
    identity: Arc<IdentityHandlers>,
    bi: Arc<BiHandlers>,
}

impl Handlers {
    pub fn new(repos: Repositories, pool: Pool, bus: EventBus, auth: Arc<AuthConfig>) -> Self {
        let estoque = Arc::new(EstoqueHandlers::new(repos.estoque, bus.clone()));

        let financeiro = Arc::new(FinanceiroHandlers::new(
            repos.contas_receber,
            repos.contas_pagar,
            bus.clone(),
        ));

        let fiscal = Arc::new(FiscalHandlers::new(
            repos.notas_fiscais,
            Arc::new(StubSefazClient),
            Arc::new(PostgresAliquotaProvider::new(pool.clone())),
            repos.tenants.clone(),
            bus.clone(),
        ));

        let identity = Arc::new(IdentityHandlers::new(
            repos.usuarios,
            repos.tenants.clone(),
            bus.clone(),
            auth.clone(),
        ));

        let backoffice = Arc::new(BackofficeHandlers::new(
            repos.backoffice,
            repos.tenants.clone(),
            identity.clone(),
            auth.clone(),
        ));

        let orcamentos = Arc::new(OrcamentosHandlers::new(
            repos.orcamentos,
            bus.clone(),
            pool.clone(),
            repos.tenants.clone(),
        ));

        Self {
            auth,
            backoffice,
            tenants: repos.tenants,
            catalogo: Arc::new(CatalogoHandlers::new(
                repos.produtos,
                repos.precificacao.clone(),
                bus.clone(),
            )),
            precificacao: repos.precificacao,
            crm: Arc::new(CrmHandlers::new(repos.clientes, bus.clone())),
            vendas: Arc::new(VendasHandlers::new(repos.vendas, bus.clone(), pool.clone())),
            fornecedores: Arc::new(FornecedoresHandlers::new(repos.fornecedores, bus.clone())),
            orcamentos,
            compras: Arc::new(ComprasHandlers::new(repos.pedidos, bus.clone(), pool.clone())),
            estoque,
            financeiro,
            fiscal,
            identity,
            bi: Arc::new(BiHandlers::new(repos.bi)),
        }
    }
}
