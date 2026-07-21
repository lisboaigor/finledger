use std::sync::Arc;

use pharos_app::{CURRENT_TENANT, TenantContext, dispatch, query_dispatch};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthConfig;
use crate::auth::jwt::{encode_backoffice_token, encode_token};
use crate::error::AppError;
use crate::identity::application::commands::RegistrarUsuario;
use crate::identity::application::handler::IdentityHandlers;
use crate::identity::application::queries::ListarUsuarios;
use crate::tenants::repository::TenantRepository;

use super::domain::{BackofficePermission, BackofficeRole, BackofficeUserResult, TenantPlan};
use super::repository::{
    BackofficeRepository, DailyRevenueResult, MonthlyRevenueResult, PlatformStatsResult,
    TenantMonthlyRevenueResult,
};

/// Minimum length for backoffice admin passwords.
const MIN_PASSWORD_LEN: usize = 8;

fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(AppError::Domain(pharos_core::DomainError::Validation(
            format!("senha deve ter pelo menos {MIN_PASSWORD_LEN} caracteres"),
        )));
    }
    Ok(())
}

fn permissions_as_strings(permissions: &[BackofficePermission]) -> Vec<String> {
    permissions.iter().map(|p| p.as_str().to_string()).collect()
}

pub struct BackofficeHandlers {
    repo: Arc<BackofficeRepository>,
    tenants: Arc<TenantRepository>,
    identity: Arc<IdentityHandlers>,
    auth: Arc<AuthConfig>,
}

#[derive(Deserialize)]
pub struct ProvisionarTenantCmd {
    pub slug: String,
    pub nome: String,
    pub admin_username: String,
    pub admin_senha: String,
}

#[derive(Serialize)]
pub struct TenantProvisionado {
    pub tenant_id: Uuid,
    pub slug: String,
}

#[derive(Deserialize)]
pub struct LoginBackofficeCmd {
    pub username: String,
    pub senha: String,
}

#[derive(Deserialize)]
pub struct CriarAdminCmd {
    pub username: String,
    pub senha: String,
    pub permissions: Vec<BackofficePermission>,
}

#[derive(Deserialize)]
pub struct AlterarPermissoesCmd {
    pub permissions: Vec<BackofficePermission>,
}

#[derive(Deserialize)]
pub struct ChangeAdminPasswordCmd {
    pub password: String,
}

#[derive(Serialize)]
pub struct AdminView {
    pub user_id: Uuid,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub ativo: bool,
}

impl From<BackofficeUserResult> for AdminView {
    fn from(r: BackofficeUserResult) -> Self {
        AdminView {
            user_id: r.user_id,
            username: r.username,
            role: r.role,
            permissions: r.permissions,
            ativo: r.ativo,
        }
    }
}

/// Revenue of one tenant enriched with its registry data, for the backoffice
/// overview page.
#[derive(Serialize)]
pub struct TenantRevenueView {
    pub tenant_id: Uuid,
    pub slug: String,
    pub nome: String,
    pub plano: String,
    pub status: String,
    pub total_cents: i64,
    pub sales_count: i64,
    pub last_30d_cents: i64,
    pub last_30d_count: i64,
    pub prev_30d_cents: i64,
    pub avg_ticket_cents: i64,
}

#[derive(Serialize)]
pub struct RevenueOverview {
    pub stats: PlatformStatsResult,
    pub tenants: Vec<TenantRevenueView>,
    pub monthly: Vec<MonthlyRevenueResult>,
    pub monthly_by_tenant: Vec<TenantMonthlyRevenueResult>,
    pub daily: Vec<DailyRevenueResult>,
}

impl BackofficeHandlers {
    pub fn new(
        repo: Arc<BackofficeRepository>,
        tenants: Arc<TenantRepository>,
        identity: Arc<IdentityHandlers>,
        auth: Arc<AuthConfig>,
    ) -> Self {
        Self {
            repo,
            tenants,
            identity,
            auth,
        }
    }

    /// Provisiona um tenant novo e seu usuário admin inicial, como uma única
    /// operação de negócio: um tenant sem admin é inacessível, então uma
    /// falha ao registrar o usuário desfaz a criação do tenant (best effort).
    pub async fn provisionar_tenant(
        &self,
        payload: ProvisionarTenantCmd,
    ) -> Result<TenantProvisionado, AppError> {
        let tenant_id = self
            .tenants
            .create_strict(&payload.slug, &payload.nome)
            .await?;

        let tenant_ctx = TenantContext::new(tenant_id);
        let cmd = RegistrarUsuario {
            username: payload.admin_username,
            senha: payload.admin_senha,
            roles: vec!["admin".to_string()],
        };
        let registered = CURRENT_TENANT
            .scope(Some(tenant_ctx), async { dispatch(&*self.identity, cmd).await })
            .await;

        if let Err(e) = registered {
            if let Err(del) = self.tenants.delete(tenant_id).await {
                tracing::error!(%tenant_id, error = %del, "failed to roll back tenant after admin creation failure");
            }
            return Err(e.into());
        }

        Ok(TenantProvisionado {
            tenant_id,
            slug: payload.slug,
        })
    }

    pub fn repo(&self) -> &BackofficeRepository {
        &self.repo
    }

    pub async fn login(&self, cmd: LoginBackofficeCmd) -> Result<String, AppError> {
        let row = self
            .repo
            .buscar_por_username(&cmd.username)
            .await?
            .filter(|r| r.ativo)
            .ok_or(AppError::Unauthorized)?;

        if !BackofficeRepository::verificar_senha(&row.password_hash, &cmd.senha) {
            return Err(AppError::Unauthorized);
        }

        let permissions = if row.role == BackofficeRole::Superadmin.as_str() {
            BackofficePermission::all_as_strings()
        } else {
            row.permissions.clone()
        };

        encode_backoffice_token(
            row.user_id,
            &row.username,
            &row.role,
            permissions,
            &self.auth.secret,
        )
    }

    pub async fn criar_admin(&self, cmd: CriarAdminCmd) -> Result<Uuid, AppError> {
        validate_password(&cmd.senha)?;
        self.repo
            .criar(
                &cmd.username,
                &cmd.senha,
                BackofficeRole::Admin,
                &permissions_as_strings(&cmd.permissions),
            )
            .await
    }

    /// Resets a support admin's password (superadmin rows are untouchable).
    pub async fn change_admin_password(
        &self,
        user_id: Uuid,
        cmd: ChangeAdminPasswordCmd,
    ) -> Result<(), AppError> {
        validate_password(&cmd.password)?;
        let hash = BackofficeRepository::hash_senha(&cmd.password)?;
        self.repo.update_password(user_id, &hash).await
    }

    pub async fn listar_admins(&self) -> Result<Vec<AdminView>, AppError> {
        self.repo
            .listar()
            .await
            .map(|rows| rows.into_iter().map(AdminView::from).collect())
    }

    pub async fn desativar_admin(&self, user_id: Uuid) -> Result<(), AppError> {
        self.repo.desativar(user_id).await
    }

    pub async fn reativar_admin(&self, user_id: Uuid) -> Result<(), AppError> {
        self.repo.reativar(user_id).await
    }

    pub async fn atualizar_tenant(&self, tenant_id: Uuid, nome: String) -> Result<(), AppError> {
        let nome = nome.trim();
        if nome.is_empty() {
            return Err(AppError::Domain(pharos_core::DomainError::Validation(
                "nome do tenant não pode ser vazio".into(),
            )));
        }
        self.tenants.atualizar_nome(tenant_id, nome).await
    }

    pub async fn alterar_permissoes(
        &self,
        user_id: Uuid,
        cmd: AlterarPermissoesCmd,
    ) -> Result<(), AppError> {
        self.repo
            .alterar_permissoes(user_id, &permissions_as_strings(&cmd.permissions))
            .await
    }

    /// Gera um token de tenant de 1 hora para acesso de suporte.
    ///
    /// O `sub` do token é o `usuario_id` de um admin real do tenant — não um
    /// UUID sintético — para que fluxos que carregam o agregado `Usuario`
    /// pelo `sub` (ex: `AlterarSenha`) continuem funcionando durante uma
    /// sessão de impersonation. O `username` claim mantém o prefixo
    /// `suporte:` para que a ação continue identificável como impersonation
    /// nos logs de auditoria.
    pub async fn impersonar_tenant(
        &self,
        tenant_id: Uuid,
        requester_username: &str,
    ) -> Result<String, AppError> {
        let tenant = self
            .tenants
            .buscar_por_id(tenant_id)
            .await?
            .ok_or(AppError::NotFound)?;

        let tenant_ctx = TenantContext::new(tenant_id);
        let usuarios: Vec<_> = CURRENT_TENANT
            .scope(Some(tenant_ctx), async {
                query_dispatch(&*self.identity, ListarUsuarios).await
            })
            .await?;

        let admin = usuarios
            .into_iter()
            .find(|u| u.ativo && u.roles.split(',').any(|r| r == "admin"))
            .ok_or_else(|| {
                AppError::Domain(pharos_core::DomainError::BusinessRule(
                    "tenant não possui um usuário admin ativo para impersonar".into(),
                ))
            })?;

        encode_token(
            admin.usuario_id,
            &format!("suporte:{requester_username}"),
            admin.roles.split(',').map(str::to_string).collect(),
            &tenant_id.to_string(),
            &tenant.slug,
            &self.auth.secret,
            1,
        )
    }

    pub async fn alterar_plano(&self, tenant_id: Uuid, plan: TenantPlan) -> Result<(), AppError> {
        self.tenants.alterar_plano(tenant_id, plan.as_str()).await
    }

    /// Cross-tenant revenue overview: platform counters, every tenant (with
    /// zeros when it has no confirmed sales), the global monthly and daily
    /// series, and the per-tenant monthly series for sparklines.
    pub async fn revenue_overview(&self, months: i32, days: i32) -> Result<RevenueOverview, AppError> {
        let tenants = self.tenants.listar().await?;
        let revenue = self.repo.revenue_by_tenant().await?;
        let monthly = self.repo.revenue_monthly(months).await?;
        let monthly_by_tenant = self.repo.revenue_monthly_by_tenant(months).await?;
        let daily = self.repo.revenue_daily(days).await?;
        let stats = self.repo.platform_stats().await?;

        let by_tenant: std::collections::HashMap<Uuid, _> =
            revenue.into_iter().map(|r| (r.tenant_id, r)).collect();

        let tenants = tenants
            .into_iter()
            .map(|t| {
                let r = by_tenant.get(&t.tenant_id);
                let total_cents = r.map_or(0, |r| r.total_cents);
                let sales_count = r.map_or(0, |r| r.sales_count);
                TenantRevenueView {
                    tenant_id: t.tenant_id,
                    slug: t.slug,
                    nome: t.nome,
                    plano: t.plano,
                    status: t.status,
                    total_cents,
                    sales_count,
                    last_30d_cents: r.map_or(0, |r| r.last_30d_cents),
                    last_30d_count: r.map_or(0, |r| r.last_30d_count),
                    prev_30d_cents: r.map_or(0, |r| r.prev_30d_cents),
                    avg_ticket_cents: if sales_count > 0 {
                        total_cents / sales_count
                    } else {
                        0
                    },
                }
            })
            .collect();

        Ok(RevenueOverview {
            stats,
            tenants,
            monthly,
            monthly_by_tenant,
            daily,
        })
    }
}
