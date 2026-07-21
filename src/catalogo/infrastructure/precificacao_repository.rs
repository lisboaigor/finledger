use chrono::{DateTime, Utc};
use pharos_postgres::Pool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::shared::tenant::current_tenant_id;

/// Configuração de precificação do catálogo: margem por categoria, custo fixo
/// por unidade (categoria/produto) e preços vistos na concorrência. Como as
/// configurações do tenant, é CRUD simples sobre o tenant da requisição atual
/// (CURRENT_TENant) — não é agregado de domínio, não passa pelo event store.
pub struct PostgresPrecificacaoRepository {
    pool: Pool,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct CategoriaMargemResult {
    pub categoria: String,
    pub margem_bps: i32,
    pub custo_fixo_unitario_centavos: Option<i64>,
}

/// Overrides de precificação por produto — prevalecem sobre categoria e
/// padrão da loja. Todos os campos são opcionais.
#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct ProdutoPrecificacaoResult {
    pub produto_id: Uuid,
    pub margem_bps: Option<i32>,
    pub custo_fixo_unitario_centavos: Option<i64>,
    pub frete_venda_bps: Option<i32>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct MaquinaCartaoResult {
    pub nome: String,
    pub taxa_bps: i32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct GiroProdutoResult {
    pub produto_id: Uuid,
    pub unidades_90d: i64,
    pub dias_sem_venda: Option<i32>,
    pub dias_desde_cadastro: i32,
    pub saldo: i32,
    /// Custo médio real do estoque — guarda do custo base na análise de
    /// preços (mesmo critério do alerta A7: max(cadastro, médio)).
    pub custo_medio_centavos: i64,
}

/// Participação do cartão na receita confirmada (90 dias) — pondera a taxa
/// da maquininha na sugestão de preço: venda em Pix/dinheiro não paga taxa.
#[derive(Serialize, sqlx::FromRow)]
pub struct MixPagamentoResult {
    pub participacao_cartao_bps: i32,
    pub amostra_vendas: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct FornecedorFreteResult {
    pub fornecedor_id: Uuid,
    pub frete_tipico_bps: i32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct PrecoConcorrenciaResult {
    pub id: Uuid,
    pub concorrente: Option<String>,
    pub preco_centavos: i64,
    pub observado_em: DateTime<Utc>,
}

/// Variação preço × vendas do último reajuste com dados suficientes —
/// insumo da frase de elasticidade mostrada no painel de precificação.
#[derive(Serialize)]
pub struct ElasticidadeResultado {
    /// Arc elasticity: (ΔQ/Q̄) / (ΔP/P̄). Negativo = demanda cai com preço.
    pub coeficiente: f64,
    pub variacao_preco_pct: f64,
    pub variacao_vendas_pct: f64,
}

/// Mínimos para o cálculo não "mentir" com pouco dado: cada período de
/// vigência precisa ter durado 14 dias e vendido 5 unidades — abaixo disso
/// devolvemos None (nenhum número é melhor que um número enganoso).
const MIN_DIAS_VIGENCIA: f64 = 14.0;
const MIN_UNIDADES: i64 = 5;

impl PostgresPrecificacaoRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    // ── Margens por categoria ────────────────────────────────────────────────

    pub async fn listar_margens(&self) -> Result<Vec<CategoriaMargemResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT categoria, margem_bps, custo_fixo_unitario_centavos
             FROM categoria_margens WHERE tenant_id = $1 ORDER BY categoria",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn definir_margem(
        &self,
        categoria: &str,
        margem_bps: i32,
        custo_fixo_unitario_centavos: Option<i64>,
    ) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query(
            "INSERT INTO categoria_margens (tenant_id, categoria, margem_bps, custo_fixo_unitario_centavos)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (tenant_id, categoria)
             DO UPDATE SET margem_bps = EXCLUDED.margem_bps,
                           custo_fixo_unitario_centavos = EXCLUDED.custo_fixo_unitario_centavos",
        )
        .bind(tenant_id)
        .bind(categoria)
        .bind(margem_bps)
        .bind(custo_fixo_unitario_centavos)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(())
    }

    pub async fn remover_margem(&self, categoria: &str) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query("DELETE FROM categoria_margens WHERE tenant_id = $1 AND categoria = $2")
            .bind(tenant_id)
            .bind(categoria)
            .execute(&self.pool)
            .await
            .map_err(AppError::infra)?
            .rows_affected();
        if n == 0 { Err(AppError::NotFound) } else { Ok(()) }
    }

    pub async fn listar_categorias(&self) -> Result<Vec<String>, AppError> {
        let tenant_id = current_tenant_id()?;
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT categoria FROM proj_produtos WHERE tenant_id = $1 ORDER BY categoria",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(rows.into_iter().map(|(c,)| c).collect())
    }

    // ── Overrides de precificação por produto ────────────────────────────────

    pub async fn listar_precificacao_produtos(
        &self,
    ) -> Result<Vec<ProdutoPrecificacaoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT produto_id, margem_bps, custo_fixo_unitario_centavos, frete_venda_bps
             FROM produto_precificacao WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Upsert dos overrides do produto; com todos os campos vazios, a linha é
    /// removida (equivale a "voltar a usar categoria/padrão da loja").
    pub async fn definir_precificacao_produto(
        &self,
        produto_id: Uuid,
        margem_bps: Option<i32>,
        custo_fixo_unitario_centavos: Option<i64>,
        frete_venda_bps: Option<i32>,
    ) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        if margem_bps.is_none() && custo_fixo_unitario_centavos.is_none() && frete_venda_bps.is_none()
        {
            sqlx::query("DELETE FROM produto_precificacao WHERE tenant_id = $1 AND produto_id = $2")
                .bind(tenant_id)
                .bind(produto_id)
                .execute(&self.pool)
                .await
                .map_err(AppError::infra)?;
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO produto_precificacao
                (tenant_id, produto_id, margem_bps, custo_fixo_unitario_centavos, frete_venda_bps)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (tenant_id, produto_id)
             DO UPDATE SET margem_bps = EXCLUDED.margem_bps,
                           custo_fixo_unitario_centavos = EXCLUDED.custo_fixo_unitario_centavos,
                           frete_venda_bps = EXCLUDED.frete_venda_bps",
        )
        .bind(tenant_id)
        .bind(produto_id)
        .bind(margem_bps)
        .bind(custo_fixo_unitario_centavos)
        .bind(frete_venda_bps)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(())
    }

    // ── Máquinas de cartão ───────────────────────────────────────────────────

    pub async fn listar_maquinas(&self) -> Result<Vec<MaquinaCartaoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT nome, taxa_bps FROM maquinas_cartao WHERE tenant_id = $1 ORDER BY nome",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn definir_maquina(&self, nome: &str, taxa_bps: i32) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query(
            "INSERT INTO maquinas_cartao (tenant_id, nome, taxa_bps) VALUES ($1, $2, $3)
             ON CONFLICT (tenant_id, nome) DO UPDATE SET taxa_bps = EXCLUDED.taxa_bps",
        )
        .bind(tenant_id)
        .bind(nome)
        .bind(taxa_bps)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(())
    }

    pub async fn remover_maquina(&self, nome: &str) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query("DELETE FROM maquinas_cartao WHERE tenant_id = $1 AND nome = $2")
            .bind(tenant_id)
            .bind(nome)
            .execute(&self.pool)
            .await
            .map_err(AppError::infra)?
            .rows_affected();
        if n == 0 { Err(AppError::NotFound) } else { Ok(()) }
    }

    // ── Giro por produto (insumo do ajuste de margem por encalhe/volume) ────

    /// Ritmo de venda e tempo parado de cada produto ativo: unidades vendidas
    /// nos últimos 90 dias, dias desde a última venda confirmada (None = nunca
    /// vendeu), dias desde o cadastro e saldo em estoque. O ajuste em si (quanto
    /// reduzir de margem para produto encalhado) é feito no frontend, junto dos
    /// demais passos da sugestão de preço — aqui só os fatos.
    pub async fn listar_giro_produtos(&self) -> Result<Vec<GiroProdutoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT p.produto_id,
                    COALESCE(v.unidades_90d, 0)::BIGINT       AS unidades_90d,
                    (CURRENT_DATE - uv.ultima_venda::date)    AS dias_sem_venda,
                    (CURRENT_DATE - p.criado_em::date)        AS dias_desde_cadastro,
                    COALESCE(s.quantidade, 0)                 AS saldo,
                    COALESCE(s.custo_medio, 0)                AS custo_medio_centavos
             FROM proj_produtos p
             LEFT JOIN proj_saldo_estoque s
                    ON s.tenant_id = p.tenant_id AND s.produto_id = p.produto_id
             LEFT JOIN LATERAL (
                 SELECT SUM(vi.quantidade) AS unidades_90d
                 FROM proj_vendas_itens vi
                 JOIN proj_vendas vd
                   ON vd.tenant_id = vi.tenant_id AND vd.venda_id = vi.venda_id
                 WHERE vi.tenant_id = p.tenant_id AND vi.produto_id = p.produto_id
                   AND vd.status = 'confirmada'
                   AND vd.confirmada_em >= now() - interval '90 days'
             ) v ON true
             LEFT JOIN LATERAL (
                 SELECT MAX(vd.confirmada_em) AS ultima_venda
                 FROM proj_vendas_itens vi
                 JOIN proj_vendas vd
                   ON vd.tenant_id = vi.tenant_id AND vd.venda_id = vi.venda_id
                 WHERE vi.tenant_id = p.tenant_id AND vi.produto_id = p.produto_id
                   AND vd.status = 'confirmada'
             ) uv ON true
             WHERE p.tenant_id = $1 AND p.ativo",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Mix de pagamento: fração da receita confirmada (90d) que passa pelo
    /// cartão. `amostra_vendas` permite ao cliente decidir se o histórico é
    /// suficiente para ponderar (senão, assume 100% no cartão — conservador).
    pub async fn mix_pagamento(&self) -> Result<MixPagamentoResult, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT COALESCE(ROUND(10000.0
                        * SUM(total_centavos) FILTER (WHERE forma_pagamento ILIKE 'Cartão%')
                        / NULLIF(SUM(total_centavos), 0)), 10000)::INT AS participacao_cartao_bps,
                    COUNT(*)::BIGINT AS amostra_vendas
             FROM proj_vendas
             WHERE tenant_id = $1 AND status = 'confirmada'
               AND confirmada_em >= NOW() - INTERVAL '90 days'",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    // ── Frete típico de compra por fornecedor ───────────────────────────────

    pub async fn listar_fretes_fornecedor(&self) -> Result<Vec<FornecedorFreteResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT fornecedor_id, frete_tipico_bps FROM fornecedor_frete WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn definir_frete_fornecedor(
        &self,
        fornecedor_id: Uuid,
        frete_tipico_bps: Option<i32>,
    ) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        match frete_tipico_bps {
            None => {
                sqlx::query(
                    "DELETE FROM fornecedor_frete WHERE tenant_id = $1 AND fornecedor_id = $2",
                )
                .bind(tenant_id)
                .bind(fornecedor_id)
                .execute(&self.pool)
                .await
                .map_err(AppError::infra)?;
            }
            Some(bps) => {
                sqlx::query(
                    "INSERT INTO fornecedor_frete (tenant_id, fornecedor_id, frete_tipico_bps)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (tenant_id, fornecedor_id)
                     DO UPDATE SET frete_tipico_bps = EXCLUDED.frete_tipico_bps",
                )
                .bind(tenant_id)
                .bind(fornecedor_id)
                .bind(bps)
                .execute(&self.pool)
                .await
                .map_err(AppError::infra)?;
            }
        }
        Ok(())
    }

    // ── Preços da concorrência ───────────────────────────────────────────────

    pub async fn listar_precos_concorrencia(
        &self,
        produto_id: Uuid,
    ) -> Result<Vec<PrecoConcorrenciaResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT id, concorrente, preco_centavos, observado_em
             FROM precos_concorrencia
             WHERE tenant_id = $1 AND produto_id = $2
             ORDER BY observado_em DESC LIMIT 20",
        )
        .bind(tenant_id)
        .bind(produto_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn registrar_preco_concorrencia(
        &self,
        produto_id: Uuid,
        concorrente: Option<&str>,
        preco_centavos: i64,
    ) -> Result<Uuid, AppError> {
        let tenant_id = current_tenant_id()?;
        let (id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO precos_concorrencia (tenant_id, produto_id, concorrente, preco_centavos)
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(tenant_id)
        .bind(produto_id)
        .bind(concorrente)
        .bind(preco_centavos)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(id)
    }

    pub async fn remover_preco_concorrencia(&self, id: Uuid) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query("DELETE FROM precos_concorrencia WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::infra)?
            .rows_affected();
        if n == 0 { Err(AppError::NotFound) } else { Ok(()) }
    }

    // ── Elasticidade de demanda ──────────────────────────────────────────────

    /// Compara o ritmo de vendas (unidades/dia) antes e depois do último
    /// reajuste de preço do produto. Devolve None quando não há dois preços
    /// distintos no histórico ou quando qualquer um dos períodos não atinge
    /// os mínimos de vigência/volume — nesses casos a UI não mostra nada.
    pub async fn elasticidade(
        &self,
        produto_id: Uuid,
    ) -> Result<Option<ElasticidadeResultado>, AppError> {
        let tenant_id = current_tenant_id()?;

        // Preço vigente (última linha do histórico).
        let atual: Option<(i64, DateTime<Utc>)> = sqlx::query_as(
            "SELECT preco_venda_centavos, vigente_desde FROM proj_historico_precos
             WHERE tenant_id = $1 AND produto_id = $2
             ORDER BY vigente_desde DESC LIMIT 1",
        )
        .bind(tenant_id)
        .bind(produto_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;
        let Some((preco_atual, vigencia_atual)) = atual else {
            return Ok(None);
        };

        // Último preço DIFERENTE antes do atual (reajustes que repetem o mesmo
        // valor não contam como mudança).
        let anterior: Option<(i64, DateTime<Utc>)> = sqlx::query_as(
            "SELECT preco_venda_centavos, vigente_desde FROM proj_historico_precos
             WHERE tenant_id = $1 AND produto_id = $2
               AND vigente_desde < $3 AND preco_venda_centavos <> $4
             ORDER BY vigente_desde DESC LIMIT 1",
        )
        .bind(tenant_id)
        .bind(produto_id)
        .bind(vigencia_atual)
        .bind(preco_atual)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;
        let Some((preco_anterior, vigencia_anterior)) = anterior else {
            return Ok(None);
        };

        let agora = Utc::now();
        let dias_anterior = (vigencia_atual - vigencia_anterior).num_seconds() as f64 / 86_400.0;
        let dias_atual = (agora - vigencia_atual).num_seconds() as f64 / 86_400.0;
        if dias_anterior < MIN_DIAS_VIGENCIA || dias_atual < MIN_DIAS_VIGENCIA {
            return Ok(None);
        }

        let qtd_anterior = self
            .unidades_vendidas(tenant_id, produto_id, vigencia_anterior, vigencia_atual)
            .await?;
        let qtd_atual = self
            .unidades_vendidas(tenant_id, produto_id, vigencia_atual, agora)
            .await?;
        if qtd_anterior < MIN_UNIDADES || qtd_atual < MIN_UNIDADES {
            return Ok(None);
        }

        // Ritmos normalizados por dia — os períodos têm durações diferentes.
        let q1 = qtd_anterior as f64 / dias_anterior;
        let q2 = qtd_atual as f64 / dias_atual;
        let p1 = preco_anterior as f64;
        let p2 = preco_atual as f64;

        // Arc elasticity: estável independentemente de qual preço é a "base".
        let var_q = (q2 - q1) / ((q2 + q1) / 2.0);
        let var_p = (p2 - p1) / ((p2 + p1) / 2.0);
        if var_p == 0.0 {
            return Ok(None);
        }

        Ok(Some(ElasticidadeResultado {
            coeficiente: var_q / var_p,
            variacao_preco_pct: (p2 / p1 - 1.0) * 100.0,
            variacao_vendas_pct: (q2 / q1 - 1.0) * 100.0,
        }))
    }

    async fn unidades_vendidas(
        &self,
        tenant_id: Uuid,
        produto_id: Uuid,
        desde: DateTime<Utc>,
        ate: DateTime<Utc>,
    ) -> Result<i64, AppError> {
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(vi.quantidade), 0)::BIGINT
             FROM proj_vendas_itens vi
             JOIN proj_vendas v ON v.tenant_id = vi.tenant_id AND v.venda_id = vi.venda_id
             WHERE vi.tenant_id = $1 AND vi.produto_id = $2
               AND v.status = 'confirmada'
               AND v.confirmada_em >= $3 AND v.confirmada_em < $4",
        )
        .bind(tenant_id)
        .bind(produto_id)
        .bind(desde)
        .bind(ate)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(total)
    }
}
