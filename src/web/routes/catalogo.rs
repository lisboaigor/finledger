use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pharos_app::{dispatch, query_dispatch};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::{AuthUser, Role, Roles};
use crate::catalogo::application::commands::{
    AtualizarPrecos, AtualizarProduto, CadastrarProduto, DesativarProduto, ReativarProduto,
};
use crate::catalogo::application::queries::{BuscarProduto, ListarProdutos, ObterElasticidade};
use crate::error::AppError;
use crate::web::{error::ApiError, state::CatalogoState};

pub async fn listar(
    State(s): State<CatalogoState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor, Role::Comprador, Role::Estoquista])?;
    let produtos = query_dispatch(&*s.catalogo, ListarProdutos).await?;
    Ok(Json(json!({ "produtos": produtos })))
}

pub async fn buscar(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor, Role::Comprador, Role::Estoquista])?;
    let produto = query_dispatch(&*s.catalogo, BuscarProduto { produto_id }).await?;
    produto
        .map(|p| Json(json!(p)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn cadastrar(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Json(cmd): Json<CadastrarProduto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_role(Roles::ADMIN | Role::Estoquista)?;
    let id = dispatch(&*s.catalogo, cmd).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "produto_id": id.to_string() })),
    ))
}

pub async fn atualizar(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
    Json(mut cmd): Json<AtualizarProduto>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    cmd.produto_id = produto_id;
    dispatch(&*s.catalogo, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn atualizar_precos(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
    Json(mut cmd): Json<AtualizarPrecos>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    cmd.produto_id = produto_id;
    dispatch(&*s.catalogo, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn desativar(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    dispatch(&*s.catalogo, DesativarProduto { produto_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reativar(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    dispatch(&*s.catalogo, ReativarProduto { produto_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Precificação assistida ────────────────────────────────────────────────────
// Configuração simples (margens por categoria, custo fixo por produto, preços
// da concorrência) — CRUD direto no repositório, sem event sourcing, mesmo
// padrão de /configuracoes. Leituras liberadas a qualquer autenticado porque o
// frontend precisa delas pra calcular a sugestão de preço.

pub async fn listar_margens(
    State(s): State<CatalogoState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let margens = s.precificacao.listar_margens().await?;
    Ok(Json(json!({ "margens": margens })))
}

#[derive(Deserialize)]
pub struct DefinirMargemBody {
    pub categoria: String,
    pub margem_bps: i32,
    #[serde(default)]
    pub custo_fixo_unitario_centavos: Option<i64>,
}

pub async fn definir_margem(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Json(body): Json<DefinirMargemBody>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    let categoria = body.categoria.trim();
    if categoria.is_empty() {
        return Err(AppError::Domain(pharos_core::DomainError::Validation(
            "Categoria não pode ser vazia".into(),
        ))
        .into());
    }
    s.precificacao
        .definir_margem(categoria, body.margem_bps, body.custo_fixo_unitario_centavos)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remover_margem(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(categoria): Path<String>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    s.precificacao.remover_margem(&categoria).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn listar_categorias(
    State(s): State<CatalogoState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let categorias = s.precificacao.listar_categorias().await?;
    Ok(Json(json!({ "categorias": categorias })))
}

pub async fn listar_precificacao_produtos(
    State(s): State<CatalogoState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let overrides = s.precificacao.listar_precificacao_produtos().await?;
    Ok(Json(json!({ "produtos": overrides })))
}

#[derive(Deserialize)]
pub struct DefinirPrecificacaoProdutoBody {
    #[serde(default)]
    pub margem_bps: Option<i32>,
    #[serde(default)]
    pub custo_fixo_unitario_centavos: Option<i64>,
    #[serde(default)]
    pub frete_venda_bps: Option<i32>,
}

pub async fn definir_precificacao_produto(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
    Json(body): Json<DefinirPrecificacaoProdutoBody>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    s.precificacao
        .definir_precificacao_produto(
            produto_id,
            body.margem_bps,
            body.custo_fixo_unitario_centavos,
            body.frete_venda_bps,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Giro por produto (unidades 90d, dias sem venda, saldo) — insumo do ajuste
/// de margem por encalhe/volume no painel de precificação e na análise de preços.
pub async fn listar_giro_produtos(
    State(s): State<CatalogoState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let giro = s.precificacao.listar_giro_produtos().await?;
    Ok(Json(json!({ "produtos": giro })))
}

/// Mix de pagamento (participação do cartão na receita 90d) — pondera a taxa
/// da maquininha na sugestão de preço.
pub async fn mix_pagamento(
    State(s): State<CatalogoState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mix = s.precificacao.mix_pagamento().await?;
    Ok(Json(json!({ "mix": mix })))
}

pub async fn listar_maquinas(
    State(s): State<CatalogoState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let maquinas = s.precificacao.listar_maquinas().await?;
    Ok(Json(json!({ "maquinas": maquinas })))
}

#[derive(Deserialize)]
pub struct DefinirMaquinaBody {
    pub nome: String,
    pub taxa_bps: i32,
}

pub async fn definir_maquina(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Json(body): Json<DefinirMaquinaBody>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    let nome = body.nome.trim();
    if nome.is_empty() {
        return Err(AppError::Domain(pharos_core::DomainError::Validation(
            "Nome da máquina não pode ser vazio".into(),
        ))
        .into());
    }
    s.precificacao.definir_maquina(nome, body.taxa_bps).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remover_maquina(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(nome): Path<String>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    s.precificacao.remover_maquina(&nome).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn listar_fretes_fornecedor(
    State(s): State<CatalogoState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let fretes = s.precificacao.listar_fretes_fornecedor().await?;
    Ok(Json(json!({ "fretes": fretes })))
}

#[derive(Deserialize)]
pub struct DefinirFreteFornecedorBody {
    #[serde(default)]
    pub frete_tipico_bps: Option<i32>,
}

pub async fn definir_frete_fornecedor(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(fornecedor_id): Path<Uuid>,
    Json(body): Json<DefinirFreteFornecedorBody>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    s.precificacao
        .definir_frete_fornecedor(fornecedor_id, body.frete_tipico_bps)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn listar_precos_concorrencia(
    State(s): State<CatalogoState>,
    _user: AuthUser,
    Path(produto_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let precos = s.precificacao.listar_precos_concorrencia(produto_id).await?;
    Ok(Json(json!({ "precos": precos })))
}

#[derive(Deserialize)]
pub struct RegistrarPrecoConcorrenciaBody {
    #[serde(default)]
    pub concorrente: Option<String>,
    pub preco_centavos: i64,
}

pub async fn registrar_preco_concorrencia(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
    Json(body): Json<RegistrarPrecoConcorrenciaBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_role(Roles::ADMIN | Role::Estoquista | Role::Vendedor)?;
    if body.preco_centavos <= 0 {
        return Err(AppError::Domain(pharos_core::DomainError::Validation(
            "Preço deve ser positivo".into(),
        ))
        .into());
    }
    let concorrente = body
        .concorrente
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty());
    let id = s
        .precificacao
        .registrar_preco_concorrencia(produto_id, concorrente, body.preco_centavos)
        .await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

pub async fn remover_preco_concorrencia(
    State(s): State<CatalogoState>,
    user: AuthUser,
    Path((_produto_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    s.precificacao.remover_preco_concorrencia(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn obter_elasticidade(
    State(s): State<CatalogoState>,
    _user: AuthUser,
    Path(produto_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let resultado = query_dispatch(&*s.catalogo, ObterElasticidade { produto_id }).await?;
    Ok(Json(json!({ "elasticidade": resultado })))
}
