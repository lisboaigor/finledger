use std::sync::Arc;

use axum::extract::FromRef;

pub use crate::bootstrap::handlers::AppState;

/// Declara um sub-state extraído de [`AppState`] via [`FromRef`], para que um
/// handler HTTP dependa apenas dos handlers de aplicação que de fato usa, em
/// vez de receber o grafo de dependências inteiro através de `State<AppState>`.
macro_rules! sub_state {
    ($name:ident { $($field:ident : $ty:ty),+ $(,)? }) => {
        #[derive(Clone)]
        pub struct $name {
            $(pub $field: $ty,)+
        }

        impl FromRef<AppState> for $name {
            fn from_ref(state: &AppState) -> Self {
                Self { $($field: state.$field.clone(),)+ }
            }
        }
    };
}

sub_state!(AuthMiddlewareState {
    auth: Arc<crate::auth::AuthConfig>,
    tenants: Arc<crate::tenants::repository::TenantRepository>,
    backoffice: Arc<crate::backoffice::handlers::BackofficeHandlers>,
});

// Consulta pública de tenant por slug — usada pelo /tls/ask (TLS on-demand do
// Caddy), pelo /tenants/{slug}/existe (composer de endereço da landing) e,
// autenticada, por /configuracoes (self-service do tenant atual).
sub_state!(TenantLookupState {
    tenants: Arc<crate::tenants::repository::TenantRepository>,
});

sub_state!(BiState {
    bi: Arc<crate::bi::application::handler::BiHandlers>,
});

sub_state!(IdentityState {
    identity: Arc<crate::identity::application::handler::IdentityHandlers>,
});

sub_state!(BackofficeState {
    backoffice: Arc<crate::backoffice::handlers::BackofficeHandlers>,
    identity: Arc<crate::identity::application::handler::IdentityHandlers>,
    tenants: Arc<crate::tenants::repository::TenantRepository>,
});

sub_state!(CatalogoState {
    catalogo: Arc<crate::catalogo::application::handler::CatalogoHandlers>,
    precificacao: Arc<crate::catalogo::infrastructure::precificacao_repository::PostgresPrecificacaoRepository>,
});

sub_state!(CrmState {
    crm: Arc<crate::crm::application::handler::CrmHandlers>,
});

sub_state!(EstoqueState {
    estoque: Arc<crate::estoque::application::handler::EstoqueHandlers>,
});

sub_state!(VendasState {
    vendas: Arc<crate::vendas::application::handler::VendasHandlers>,
});

sub_state!(FornecedoresState {
    fornecedores: Arc<crate::fornecedores::application::handler::FornecedoresHandlers>,
});

sub_state!(OrcamentosState {
    orcamentos: Arc<crate::orcamentos::application::handler::OrcamentosHandlers>,
});

sub_state!(ComprasState {
    compras: Arc<crate::compras::application::handler::ComprasHandlers>,
});

sub_state!(FinanceiroState {
    financeiro: Arc<crate::financeiro::application::handler::FinanceiroHandlers>,
});

sub_state!(FiscalState {
    fiscal: Arc<
        crate::fiscal::application::handler::FiscalHandlers<
            crate::fiscal::infrastructure::sefaz::StubSefazClient,
        >,
    >,
});
