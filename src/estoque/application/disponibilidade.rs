use pharos_postgres::Pool;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::domain::Disponibilidade;
use crate::shared::tenant::current_tenant_id;

/// Resolve a disponibilidade de um produto para venda/orçamento (I/O — lê
/// `proj_produtos`/`proj_saldo_estoque` diretamente, mesmo padrão de leitura
/// cross-contexto usado em `FiscalHandlers::enriquecer_itens`, em vez de
/// depender do repositório do catálogo só para esta checagem pontual).
///
/// A regra de negócio em si (comparar quantidade pretendida × saldo,
/// somando o que já está no documento) NÃO vive aqui — vive em
/// `Disponibilidade::validar`, chamada de dentro do agregado `Venda`/
/// `Orcamento`. Esta função só busca os dados e monta o value object.
///
/// `ignorar_checagem` cobre tanto a venda sob encomenda (confirmação
/// explícita do vendedor, por item) quanto a feature flag de orçamentos
/// (por tenant) — cabe ao chamador decidir quando ela vale `true`.
pub async fn resolver_disponibilidade(
    pool: &Pool,
    produto_id: Uuid,
    ignorar_checagem: bool,
) -> Result<Disponibilidade, AppError> {
    if ignorar_checagem {
        return Ok(Disponibilidade::SemChecagem);
    }

    let tenant_id = current_tenant_id()?;

    let controla: Option<bool> = sqlx::query_scalar(
        "SELECT controla_estoque FROM proj_produtos WHERE produto_id = $1 AND tenant_id = $2",
    )
    .bind(produto_id)
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::infra)?;

    if !controla.unwrap_or(true) {
        return Ok(Disponibilidade::NaoControlada);
    }

    let saldo: Option<i32> = sqlx::query_scalar(
        "SELECT quantidade FROM proj_saldo_estoque WHERE produto_id = $1 AND tenant_id = $2",
    )
    .bind(produto_id)
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::infra)?;

    Ok(Disponibilidade::Controlada(
        saldo.unwrap_or(0).max(0) as u32
    ))
}
