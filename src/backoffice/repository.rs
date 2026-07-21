use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::NaiveDate;
use pharos_postgres::Pool;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;

use super::domain::{BackofficeRole, BackofficeUserResult};

/// Confirmed-sales revenue of one tenant, produced by the SECURITY DEFINER
/// function `backoffice_revenue_by_tenant` (the only cross-tenant view of
/// `proj_vendas` the app role is allowed).
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TenantRevenueResult {
    pub tenant_id: Uuid,
    pub total_cents: i64,
    pub sales_count: i64,
    pub last_30d_cents: i64,
    pub last_30d_count: i64,
    /// Revenue of the 30-day window before the last one (days 30–60 ago),
    /// used to compute growth.
    pub prev_30d_cents: i64,
}

/// One month of global confirmed-sales revenue (all tenants combined).
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MonthlyRevenueResult {
    pub month: NaiveDate,
    pub total_cents: i64,
    pub sales_count: i64,
}

/// One month of one tenant's confirmed-sales revenue (table sparklines).
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TenantMonthlyRevenueResult {
    pub tenant_id: Uuid,
    pub month: NaiveDate,
    pub total_cents: i64,
}

/// One day of global confirmed-sales revenue.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DailyRevenueResult {
    pub day: NaiveDate,
    pub total_cents: i64,
    pub sales_count: i64,
}

/// Platform-wide operational counters.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PlatformStatsResult {
    pub total_users: i64,
    pub active_users: i64,
    pub total_products: i64,
    pub total_clients: i64,
    pub today_cents: i64,
    pub today_count: i64,
}

/// Every mutation below that targets a support admin by `user_id` must
/// exclude superadmin rows — those credentials are managed via seed env vars
/// only. Centralized here so the guard can't drift out of sync between the
/// four call sites that need it (each previously spelled its own condition).
const APENAS_ADMIN_DE_SUPORTE: &str = "role != 'superadmin'";

pub struct BackofficeRepository {
    pool: Pool,
}

impl BackofficeRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn criar(
        &self,
        username: &str,
        senha: &str,
        role: BackofficeRole,
        permissions: &[String],
    ) -> Result<Uuid, AppError> {
        let hash = Self::hash_senha(senha)?;

        let row: (Uuid,) = sqlx::query_as(
            "INSERT INTO backoffice_users (username, password_hash, role, permissions)
             VALUES ($1, $2, $3, $4)
             RETURNING user_id",
        )
        .bind(username)
        .bind(&hash)
        .bind(role.as_str())
        .bind(permissions)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::infra)?;

        Ok(row.0)
    }

    /// Whether the admin row is still active, or `None` if it no longer
    /// exists. Used by `require_backoffice_auth` to revalidate on every
    /// request — a long-lived backoffice token must stop working the moment
    /// the account is deactivated, not just wait for it to expire.
    pub async fn esta_ativo(&self, user_id: Uuid) -> Result<Option<bool>, AppError> {
        sqlx::query_scalar("SELECT ativo FROM backoffice_users WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::infra)
    }

    pub async fn buscar_por_username(
        &self,
        username: &str,
    ) -> Result<Option<BackofficeUserResult>, AppError> {
        sqlx::query_as(
            "SELECT user_id, username, password_hash, role, permissions, ativo, criado_em
             FROM backoffice_users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn listar(&self) -> Result<Vec<BackofficeUserResult>, AppError> {
        sqlx::query_as(
            "SELECT user_id, username, password_hash, role, permissions, ativo, criado_em
             FROM backoffice_users ORDER BY criado_em",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn desativar(&self, user_id: Uuid) -> Result<(), AppError> {
        // The WHERE fragment is a fixed compile-time constant, never user
        // input — `AssertSqlSafe` is sqlx's sanctioned escape hatch for this.
        let n = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE backoffice_users SET ativo = FALSE
             WHERE user_id = $1 AND {APENAS_ADMIN_DE_SUPORTE}"
        )))
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?
        .rows_affected();

        if n == 0 {
            Err(AppError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn reativar(&self, user_id: Uuid) -> Result<(), AppError> {
        let n = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE backoffice_users SET ativo = TRUE
             WHERE user_id = $1 AND {APENAS_ADMIN_DE_SUPORTE}"
        )))
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?
        .rows_affected();

        if n == 0 {
            Err(AppError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn alterar_permissoes(
        &self,
        user_id: Uuid,
        permissions: &[String],
    ) -> Result<(), AppError> {
        let n = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE backoffice_users SET permissions = $1
             WHERE user_id = $2 AND {APENAS_ADMIN_DE_SUPORTE}"
        )))
        .bind(permissions)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?
        .rows_affected();

        if n == 0 {
            Err(AppError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn revenue_by_tenant(&self) -> Result<Vec<TenantRevenueResult>, AppError> {
        sqlx::query_as("SELECT * FROM backoffice_revenue_by_tenant()")
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::infra)
    }

    pub async fn revenue_monthly(&self, months: i32) -> Result<Vec<MonthlyRevenueResult>, AppError> {
        sqlx::query_as("SELECT * FROM backoffice_revenue_monthly($1)")
            .bind(months)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::infra)
    }

    pub async fn revenue_monthly_by_tenant(
        &self,
        months: i32,
    ) -> Result<Vec<TenantMonthlyRevenueResult>, AppError> {
        sqlx::query_as("SELECT * FROM backoffice_revenue_monthly_by_tenant($1)")
            .bind(months)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::infra)
    }

    pub async fn revenue_daily(&self, days: i32) -> Result<Vec<DailyRevenueResult>, AppError> {
        sqlx::query_as("SELECT * FROM backoffice_revenue_daily($1)")
            .bind(days)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::infra)
    }

    pub async fn platform_stats(&self) -> Result<PlatformStatsResult, AppError> {
        sqlx::query_as("SELECT * FROM backoffice_platform_stats()")
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::infra)
    }

    /// Updates a support admin's password hash. Superadmin rows are protected —
    /// their credentials are managed via the seed env vars only.
    pub async fn update_password(&self, user_id: Uuid, password_hash: &str) -> Result<(), AppError> {
        let n = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE backoffice_users SET password_hash = $1
             WHERE user_id = $2 AND {APENAS_ADMIN_DE_SUPORTE}"
        )))
        .bind(password_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?
        .rows_affected();

        if n == 0 {
            Err(AppError::NotFound)
        } else {
            Ok(())
        }
    }

    pub fn hash_senha(senha: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(senha.as_bytes(), &salt)
            .map_err(|e| AppError::infra(format!("hash: {e}")))
            .map(|h| h.to_string())
    }

    pub fn verificar_senha(hash: &str, senha: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(senha.as_bytes(), &parsed)
            .is_ok()
    }
}
