use std::sync::Arc;

use anyhow::Result;
use pharos_app::{CURRENT_TENANT, TenantContext, dispatch};
use tracing::info;

use crate::backoffice::handlers::BackofficeHandlers;
use crate::identity::application::commands::RegistrarUsuario;
use crate::tenants::repository::TenantRepository;

pub async fn seed_demo_tenant(
    tenants: &Arc<TenantRepository>,
    identity: &Arc<crate::identity::application::handler::IdentityHandlers>,
) -> Result<()> {
    // Cria o tenant `demo` com usuário admin/admin — conveniência de desenvolvimento.
    // NUNCA deve rodar em produção (porta dos fundos), por isso é gated por env e
    // desligado por padrão. Habilite com SEED_DEMO_TENANT=1|true apenas em dev.
    if !matches!(
        std::env::var("SEED_DEMO_TENANT").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    ) {
        info!("SEED_DEMO_TENANT desativado — seed do tenant demo ignorado");
        return Ok(());
    }

    let tenant_id = tenants.criar("demo", "Empresa Finledger Demo").await?;
    info!(%tenant_id, slug = "demo", "tenant demo garantido");

    let tenant_ctx = TenantContext::new(tenant_id);
    let cmd = RegistrarUsuario {
        username: "admin".to_string(),
        senha: "admin".to_string(),
        roles: vec!["admin".to_string()],
    };

    CURRENT_TENANT
        .scope(Some(tenant_ctx), async {
            match dispatch(identity.as_ref(), cmd).await {
                Ok(id) => info!(%id, "usuário admin do tenant demo criado"),
                Err(_) => info!("usuário admin do tenant demo já existe, ignorando seed"),
            }
        })
        .await;

    Ok(())
}

pub async fn seed_superadmin(backoffice: &Arc<BackofficeHandlers>) -> Result<()> {
    // `ok().filter(...)` trata "ausente" e "definido mas vazio" da mesma forma — o
    // segundo é comum em orquestradores (docker-compose passa `${VAR:-}` como "").
    let username = std::env::var("SUPERADMIN_USERNAME")
        .ok()
        .filter(|u| !u.trim().is_empty());
    let senha = std::env::var("SUPERADMIN_PASSWORD")
        .ok()
        .filter(|p| !p.is_empty());
    let (Some(username), Some(senha)) = (username, senha) else {
        info!("SUPERADMIN_USERNAME/PASSWORD não definidos — seed de superadmin ignorado");
        return Ok(());
    };

    use crate::backoffice::domain::BackofficeRole;

    let repo = backoffice.repo();
    match repo
        .criar(&username, &senha, BackofficeRole::Superadmin, &[])
        .await
    {
        Ok(id) => info!(%id, %username, "superadmin criado"),
        Err(_) => info!(%username, "superadmin já existe, ignorando seed"),
    }

    Ok(())
}
