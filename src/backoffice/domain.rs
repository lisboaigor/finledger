use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BackofficeUserResult {
    pub user_id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub ativo: bool,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackofficeRole {
    Superadmin,
    Admin,
}

impl BackofficeRole {
    pub fn as_str(self) -> &'static str {
        match self {
            BackofficeRole::Superadmin => "superadmin",
            BackofficeRole::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackofficePermission {
    #[serde(rename = "tenants:read")]
    TenantsRead,
    #[serde(rename = "tenants:write")]
    TenantsWrite,
    #[serde(rename = "tenants:impersonate")]
    TenantsImpersonate,
    #[serde(rename = "admins:manage")]
    AdminsManage,
}

impl BackofficePermission {
    pub const ALL: [Self; 4] = [
        Self::TenantsRead,
        Self::TenantsWrite,
        Self::TenantsImpersonate,
        Self::AdminsManage,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::TenantsRead => "tenants:read",
            Self::TenantsWrite => "tenants:write",
            Self::TenantsImpersonate => "tenants:impersonate",
            Self::AdminsManage => "admins:manage",
        }
    }

    /// String form of every permission — what gets persisted and encoded in JWTs.
    pub fn all_as_strings() -> Vec<String> {
        Self::ALL.iter().map(|p| p.as_str().to_string()).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TenantPlan {
    Basico,
    Profissional,
    Enterprise,
}

impl TenantPlan {
    pub fn as_str(self) -> &'static str {
        match self {
            TenantPlan::Basico => "basico",
            TenantPlan::Profissional => "profissional",
            TenantPlan::Enterprise => "enterprise",
        }
    }
}
