pub mod jwt;
pub mod middleware;

use std::{fmt, ops::BitOr};

use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;

use crate::auth::jwt::{BackofficeClaims, Claims};
use crate::backoffice::domain::{BackofficePermission, BackofficeRole};
use crate::error::AppError;

pub struct AuthConfig {
    pub secret: String,
}

impl AuthConfig {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

/// Roles do sistema. Qualquer string desconhecida no JWT é descartada silenciosamente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Admin,
    Vendedor,
    Comprador,
    Estoquista,
    Financeiro,
    Fiscal,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Role::Admin => "admin",
            Role::Vendedor => "vendedor",
            Role::Comprador => "comprador",
            Role::Estoquista => "estoquista",
            Role::Financeiro => "financeiro",
            Role::Fiscal => "fiscal",
        };
        f.write_str(s)
    }
}

impl TryFrom<&str> for Role {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "admin" => Ok(Role::Admin),
            "vendedor" => Ok(Role::Vendedor),
            "comprador" => Ok(Role::Comprador),
            "estoquista" => Ok(Role::Estoquista),
            "financeiro" => Ok(Role::Financeiro),
            "fiscal" => Ok(Role::Fiscal),
            _ => Err(()),
        }
    }
}

pub struct Roles(u64);

impl Roles {
    pub const EMPTY: Self = Self(0);

    pub const ADMIN: Self = Self(1 << Role::Admin as u64);
    pub const VENDEDOR: Self = Self(1 << Role::Vendedor as u64);
    pub const COMPRADOR: Self = Self(1 << Role::Comprador as u64);
    pub const ESTOQUISTA: Self = Self(1 << Role::Estoquista as u64);
    pub const FINANCEIRO: Self = Self(1 << Role::Financeiro as u64);
    pub const FISCAL: Self = Self(1 << Role::Fiscal as u64);

    pub fn contains(&self, role: Role) -> bool {
        self.0 & (1 << role as u64) != 0
    }

    pub fn intersects(&self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<Role> for Roles {
    fn from(role: Role) -> Self {
        Self(1 << role as u8)
    }
}

impl BitOr for Role {
    type Output = Roles;

    fn bitor(self, rhs: Self) -> Self::Output {
        Roles::from(self) | Roles::from(rhs)
    }
}

impl BitOr<Role> for Roles {
    type Output = Roles;

    fn bitor(self, rhs: Role) -> Self::Output {
        self | Roles::from(rhs)
    }
}

impl BitOr for Roles {
    type Output = Roles;

    fn bitor(self, rhs: Roles) -> Roles {
        Roles(self.0 | rhs.0)
    }
}

/// Extrator que disponibiliza o usuário autenticado nos handlers.
/// Requer que o middleware `require_auth` já tenha inserido os `Claims` nas extensions.
#[derive(Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub username: String,
    pub roles: Vec<Role>,
    /// Tenant (UUID em string) dono dos dados desta requisição.
    pub tenant_id: String,
}

impl AuthUser {
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    pub fn has_roles(&self, roles: Roles) -> bool {
        self.roles.iter().any(|r| roles.contains(r.clone()))
    }

    /// Retorna `Err(AppError::Forbidden)` se o usuário não tiver o role exigido.
    pub fn exigir_role(&self, roles: Roles) -> Result<(), AppError> {
        if self.has_roles(roles) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    /// Retorna `Err(AppError::Forbidden)` se o usuário não tiver nenhum dos roles listados.
    /// `Role::Admin` sempre passa (acesso total).
    pub fn exigir_qualquer_role(&self, roles: &[Role]) -> Result<(), AppError> {
        if self.has_role(&Role::Admin) || roles.iter().any(|r| self.has_role(r)) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts.extensions.get::<Claims>().ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "não autenticado" })),
            )
                .into_response()
        })?;

        let id = Uuid::parse_str(&claims.sub).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "token malformado" })),
            )
                .into_response()
        })?;

        let roles = claims
            .roles
            .iter()
            .filter_map(|s| Role::try_from(s.as_str()).ok())
            .collect();

        Ok(AuthUser {
            id,
            username: claims.username.clone(),
            roles,
            tenant_id: claims.tenant_id.clone(),
        })
    }
}

/// Usuário autenticado do backoffice (superadmin ou admin de suporte).
/// Requer que o middleware `require_backoffice_auth` tenha inserido `BackofficeClaims` nas extensions.
#[derive(Clone)]
pub struct BackofficeUser {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
}

impl BackofficeUser {
    pub fn is_superadmin(&self) -> bool {
        self.role == BackofficeRole::Superadmin.as_str()
    }

    pub fn exigir_superadmin(&self) -> Result<(), AppError> {
        if self.is_superadmin() {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    pub fn tem_permissao(&self, perm: BackofficePermission) -> bool {
        self.is_superadmin() || self.permissions.iter().any(|p| p == perm.as_str())
    }

    pub fn exigir_permissao(&self, perm: BackofficePermission) -> Result<(), AppError> {
        if self.tem_permissao(perm) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

impl<S> FromRequestParts<S> for BackofficeUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts.extensions.get::<BackofficeClaims>().ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "não autenticado no backoffice" })),
            )
                .into_response()
        })?;

        let id = Uuid::parse_str(&claims.sub).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "token malformado" })),
            )
                .into_response()
        })?;

        Ok(BackofficeUser {
            id,
            username: claims.username.clone(),
            role: claims.role.clone(),
            permissions: claims.permissions.clone(),
        })
    }
}
