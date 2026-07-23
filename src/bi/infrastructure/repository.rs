use pharos_postgres::Pool;
use uuid::Uuid;

use crate::bi::application::queries::{
    AgingResult, AlertaResult, BiResumoResult, CategoriaGiroResult, CicloFinanceiroResult, ClienteRiscoResult,
    DevedorResult, EstoqueMortoResult, FunilResult, MatrizAbcXyzResult, OrcamentoExpirandoResult,
    PedidoParadoResult, ReceitaDiaResult, RfmSegmentoResult, RupturaResult, SemanaFluxoResult, VendedorResult,
};
use crate::error::AppError;

/// Leituras do warehouse `bi` e das projeções, sempre no contexto do tenant da
/// requisição (a RLS filtra pela GUC `app.tenant_id` gravada pelo pool).
pub struct PostgresBiRepository {
    pool: Pool,
}

impl PostgresBiRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Score de saúde 0–100 com detalhamento por componente. A função é
    /// SECURITY DEFINER (varre fora da RLS), por isso recebe o tenant explícito.
    pub async fn score_saude(&self) -> Result<serde_json::Value, AppError> {
        let tenant_id = crate::shared::tenant::current_tenant_id()?;
        sqlx::query_scalar("SELECT bi.score_saude($1)")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::infra)
    }

    /// Meta de faturamento do mês definida em Configurações (para o card de
    /// progresso do dashboard).
    pub async fn meta_faturamento(&self) -> Result<Option<i64>, AppError> {
        let tenant_id = crate::shared::tenant::current_tenant_id()?;
        let meta: Option<Option<i64>> = sqlx::query_scalar(
            "SELECT meta_faturamento_mensal_centavos FROM tenants WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(meta.flatten())
    }

    /// Timestamp do último ciclo de ETL bem-sucedido (`bi.watermarks`, chave
    /// `etl_ciclo`); `None` enquanto o primeiro ciclo não roda.
    pub async fn etl_atualizado_em(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
        sqlx::query_scalar("SELECT ultimo FROM bi.watermarks WHERE tabela = 'etl_ciclo'")
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::infra)
    }

    pub async fn resumo(&self) -> Result<BiResumoResult, AppError> {
        // Cortes de mês/dia no fuso do negócio (bi.data_local — issue #12).
        sqlx::query_as(
            r#"
            SELECT
                -- Faturamento DATADO e líquido: lê o fato do BI (bruto por mês da
                -- venda + negativo datado da devolução, já sem o desconto global),
                -- não proj_vendas — cujo total é recalculado na devolução e faria
                -- um mês fechado mudar retroativamente (issue #17).
                (SELECT COALESCE(SUM(receita_centavos), 0) FROM bi.fato_vendas_item
                  WHERE status IN ('confirmada', 'devolucao')
                    AND data_venda
                        >= date_trunc('month', now() AT TIME ZONE 'America/Sao_Paulo')::date)::bigint
                    AS receita_mes_centavos,
                (SELECT COALESCE(SUM(receita_centavos), 0) FROM bi.fato_vendas_item
                  WHERE status IN ('confirmada', 'devolucao')
                    AND data_venda
                        >= (date_trunc('month', now() AT TIME ZONE 'America/Sao_Paulo') - INTERVAL '1 month')::date
                    AND data_venda
                        <  date_trunc('month', now() AT TIME ZONE 'America/Sao_Paulo')::date)::bigint
                    AS receita_mes_anterior_centavos,
                (SELECT COALESCE(SUM(valor_original - valor_recebido), 0) FROM proj_contas_receber
                  WHERE status IN ('pendente', 'parcial') AND vencimento < NOW())::bigint
                    AS vencidas_centavos,
                -- "Esperado em 30 dias": só recebíveis A VENCER na janela — as
                -- vencidas ficam exclusivamente no card de atrasados (issue #15).
                -- Contas a PAGAR vencidas continuam entrando: são saída certa.
                ((SELECT COALESCE(SUM(valor_original - valor_recebido), 0) FROM proj_contas_receber
                   WHERE status IN ('pendente', 'parcial')
                     AND vencimento >= NOW() AND vencimento <= NOW() + INTERVAL '30 days')
                 - (SELECT COALESCE(SUM(valor_original - valor_pago), 0) FROM proj_contas_pagar
                     WHERE status IN ('pendente', 'parcial') AND vencimento <= NOW() + INTERVAL '30 days'))::bigint
                    AS caixa_30d_centavos,
                -- Margem também datada e líquida de devoluções (net do mês).
                (SELECT SUM(margem_centavos)::float8 / NULLIF(SUM(receita_centavos), 0)::float8 * 100
                   FROM bi.fato_vendas_item
                  WHERE status IN ('confirmada', 'devolucao')
                    AND data_venda >= date_trunc('month', now() AT TIME ZONE 'America/Sao_Paulo')::date)
                    AS margem_percent,
                (SELECT SUM(margem_liquida_centavos)::float8 / NULLIF(SUM(receita_centavos), 0)::float8 * 100
                   FROM bi.fato_vendas_item
                  WHERE status IN ('confirmada', 'devolucao')
                    AND data_venda >= date_trunc('month', now() AT TIME ZONE 'America/Sao_Paulo')::date)
                    AS margem_liquida_percent,
                (SELECT (COUNT(*) FILTER (WHERE status = 'convertido'))::float8
                        / NULLIF(COUNT(*) FILTER (WHERE status IN ('aceito', 'recusado', 'expirado', 'convertido')), 0)::float8
                        * 100
                   FROM proj_orcamentos
                  WHERE atualizado_em >= NOW() - INTERVAL '90 days')
                    AS conversao_percent
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn receita_diaria(&self) -> Result<Vec<ReceitaDiaResult>, AppError> {
        sqlx::query_as(
            r#"
            -- Gráfico diário: fato datado e líquido (venda no seu dia + devolução
            -- no dia dela), não proj_vendas recalculado na devolução (issue #17).
            SELECT to_char(d, 'YYYY-MM-DD') AS dia,
                   COALESCE(SUM(f.receita_centavos), 0)::bigint AS total_centavos
              FROM generate_series(bi.data_local(now()) - 29, bi.data_local(now()), '1 day') AS d
              LEFT JOIN bi.fato_vendas_item f
                     ON f.status IN ('confirmada', 'devolucao') AND f.data_venda = d::date
             GROUP BY d
             ORDER BY d
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn alertas(&self, limite: i64) -> Result<Vec<AlertaResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT alerta_id, codigo, titulo, mensagem, link,
                   impacto_centavos, urgencia_dias, score::float8 AS score, status
              FROM bi.alertas
             WHERE status IN ('novo', 'visto')
             ORDER BY score DESC, impacto_centavos DESC
             LIMIT $1
            "#,
        )
        .bind(limite)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    // ── Financeiro / Caixa ────────────────────────────────────────────────────

    /// DSO/DIO/DPO pela fórmula contábil simples (saldo aberto ÷ média diária
    /// dos últimos 90 dias); zeros quando não há base de cálculo.
    pub async fn ciclo_financeiro(&self) -> Result<CicloFinanceiroResult, AppError> {
        sqlx::query_as(
            r#"
            WITH receita AS (
                SELECT COALESCE(SUM(total_centavos), 0)::numeric / 90 AS dia
                  FROM proj_vendas
                 WHERE status = 'confirmada' AND confirmada_em >= NOW() - INTERVAL '90 days'),
            cmv AS (
                SELECT COALESCE(SUM(custo_centavos), 0)::numeric / 90 AS dia
                  FROM bi.fato_vendas_item
                 WHERE status = 'confirmada' AND data_venda >= bi.data_local(now()) - 90),
            compras AS (
                SELECT COALESCE(SUM(total_centavos), 0)::numeric / 90 AS dia
                  FROM proj_pedidos_compra
                 WHERE status IN ('recebido_parcial', 'recebido_total')
                   AND atualizado_em >= NOW() - INTERVAL '90 days'),
            cr AS (SELECT COALESCE(SUM(valor_original - valor_recebido), 0)::numeric AS v
                     FROM proj_contas_receber WHERE status IN ('pendente', 'parcial')),
            cp AS (SELECT COALESCE(SUM(valor_original - valor_pago), 0)::numeric AS v
                     FROM proj_contas_pagar WHERE status IN ('pendente', 'parcial')),
            est AS (SELECT COALESCE(SUM(quantidade::bigint * custo_medio), 0)::numeric AS v
                      FROM proj_saldo_estoque)
            SELECT COALESCE(ROUND(cr.v / NULLIF(receita.dia, 0), 1), 0)::float8 AS dso,
                   COALESCE(ROUND(est.v / NULLIF(cmv.dia, 0), 1), 0)::float8    AS dio,
                   COALESCE(ROUND(cp.v / NULLIF(compras.dia, 0), 1), 0)::float8 AS dpo,
                   (COALESCE(ROUND(cr.v / NULLIF(receita.dia, 0), 1), 0)
                    + COALESCE(ROUND(est.v / NULLIF(cmv.dia, 0), 1), 0)
                    - COALESCE(ROUND(cp.v / NULLIF(compras.dia, 0), 1), 0))::float8 AS ccc
              FROM receita, cmv, compras, cr, cp, est
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn aging_recebiveis(&self) -> Result<Vec<AgingResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT faixa, COUNT(*)::bigint AS quantidade, COALESCE(SUM(saldo), 0)::bigint AS total_centavos
              FROM (SELECT valor_original - valor_recebido AS saldo,
                           CASE WHEN vencimento >= NOW()                       THEN 'A vencer'
                                WHEN vencimento >= NOW() - INTERVAL '30 days'  THEN '1–30 dias'
                                WHEN vencimento >= NOW() - INTERVAL '60 days'  THEN '31–60 dias'
                                WHEN vencimento >= NOW() - INTERVAL '90 days'  THEN '61–90 dias'
                                ELSE 'Mais de 90 dias' END AS faixa,
                           CASE WHEN vencimento >= NOW()                       THEN 0
                                WHEN vencimento >= NOW() - INTERVAL '30 days'  THEN 1
                                WHEN vencimento >= NOW() - INTERVAL '60 days'  THEN 2
                                WHEN vencimento >= NOW() - INTERVAL '90 days'  THEN 3
                                ELSE 4 END AS ordem
                      FROM proj_contas_receber
                     WHERE status IN ('pendente', 'parcial')) s
             GROUP BY faixa
             ORDER BY MIN(ordem)
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Fluxo operacional projetado por semana (12 semanas): CR − CP a vencer.
    pub async fn projecao_semanal(&self) -> Result<Vec<SemanaFluxoResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT to_char((NOW() + s.w * INTERVAL '7 days')::date, 'DD/MM') AS semana,
                   (SELECT COALESCE(SUM(valor_original - valor_recebido), 0)
                      FROM proj_contas_receber
                     WHERE status IN ('pendente', 'parcial')
                       AND vencimento >= NOW() + s.w * INTERVAL '7 days'
                       AND vencimento <  NOW() + (s.w + 1) * INTERVAL '7 days')::bigint AS receber_centavos,
                   (SELECT COALESCE(SUM(valor_original - valor_pago), 0)
                      FROM proj_contas_pagar
                     WHERE status IN ('pendente', 'parcial')
                       AND vencimento >= NOW() + s.w * INTERVAL '7 days'
                       AND vencimento <  NOW() + (s.w + 1) * INTERVAL '7 days')::bigint AS pagar_centavos
              FROM generate_series(0, 11) AS s(w)
             ORDER BY s.w
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn top_devedores(&self) -> Result<Vec<DevedorResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT c.cliente_id,
                   COALESCE(cl.nome, 'Consumidor não identificado') AS nome,
                   SUM(c.valor_original - c.valor_recebido)::bigint AS saldo_centavos,
                   MAX(EXTRACT(day FROM NOW() - c.vencimento))::int AS dias_atraso
              FROM proj_contas_receber c
              LEFT JOIN proj_clientes cl
                     ON cl.tenant_id = c.tenant_id AND cl.cliente_id = c.cliente_id
             WHERE c.status IN ('pendente', 'parcial') AND c.vencimento < NOW()
             GROUP BY c.cliente_id, cl.nome
             ORDER BY saldo_centavos DESC
             LIMIT 10
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    // ── Comercial / Funil / Clientes ──────────────────────────────────────────

    pub async fn funil_orcamentos(&self) -> Result<Vec<FunilResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT status, COUNT(*)::bigint AS quantidade,
                   COALESCE(SUM(total_centavos), 0)::bigint AS total_centavos
              FROM proj_orcamentos
             WHERE criado_em >= NOW() - INTERVAL '90 days'
             GROUP BY status
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn orcamentos_expirando(&self) -> Result<Vec<OrcamentoExpirandoResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT o.orcamento_id,
                   COALESCE(cl.nome, 'Cliente não identificado') AS cliente,
                   o.total_centavos,
                   GREATEST(EXTRACT(day FROM (o.criado_em + o.validade_dias * INTERVAL '1 day') - NOW()), 0)::int
                       AS vence_em_dias
              FROM proj_orcamentos o
              LEFT JOIN proj_clientes cl
                     ON cl.tenant_id = o.tenant_id AND cl.cliente_id = o.cliente_id
             WHERE o.status = 'emitido'
               AND o.criado_em + o.validade_dias * INTERVAL '1 day' <= NOW() + INTERVAL '3 days'
             ORDER BY o.total_centavos DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn desempenho_vendedores(&self) -> Result<Vec<VendedorResult>, AppError> {
        sqlx::query_as(
            r#"
            WITH v AS (
                SELECT vendedor_id, COUNT(*)::bigint AS n, SUM(total_centavos) AS receita
                  FROM proj_vendas
                 WHERE status = 'confirmada' AND confirmada_em >= NOW() - INTERVAL '90 days'
                 GROUP BY vendedor_id),
            o AS (
                SELECT vendedor_id,
                       COUNT(*) FILTER (WHERE status = 'convertido')::float8 AS convertidos,
                       COUNT(*) FILTER (WHERE status IN ('aceito', 'recusado', 'expirado', 'convertido'))::float8
                           AS decididos,
                       SUM(desconto_centavos) FILTER (WHERE status = 'convertido') AS desconto,
                       SUM(total_centavos + desconto_centavos) FILTER (WHERE status = 'convertido') AS base
                  FROM proj_orcamentos
                 WHERE atualizado_em >= NOW() - INTERVAL '90 days'
                 GROUP BY vendedor_id)
            SELECT u.username AS vendedor,
                   COALESCE(v.receita, 0)::bigint AS receita_centavos,
                   COALESCE(v.n, 0)::bigint AS vendas,
                   CASE WHEN COALESCE(v.n, 0) > 0 THEN (v.receita / v.n)::bigint ELSE 0 END AS ticket_centavos,
                   CASE WHEN o.decididos > 0 THEN o.convertidos / o.decididos * 100 END AS conversao_percent,
                   CASE WHEN o.base > 0 THEN o.desconto::float8 / o.base * 100 END AS desconto_percent
              FROM proj_usuarios u
              LEFT JOIN v ON v.vendedor_id = u.usuario_id
              LEFT JOIN o ON o.vendedor_id = u.usuario_id
             WHERE COALESCE(v.n, 0) > 0 OR COALESCE(o.decididos, 0) > 0
             ORDER BY receita_centavos DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn rfm_segmentos(&self) -> Result<Vec<RfmSegmentoResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT segmento, COUNT(*)::bigint AS clientes,
                   COALESCE(SUM(valor_12m), 0)::bigint AS valor_centavos
              FROM bi.analise_clientes
             GROUP BY segmento
             ORDER BY valor_centavos DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn clientes_em_risco(&self) -> Result<Vec<ClienteRiscoResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT a.cliente_id, a.nome,
                   a.valor_12m AS valor_12m_centavos,
                   a.recencia_dias,
                   cl.telefone, cl.email
              FROM bi.analise_clientes a
              JOIN proj_clientes cl
                    ON cl.tenant_id = a.tenant_id AND cl.cliente_id = a.cliente_id
             WHERE a.segmento = 'Em risco'
             ORDER BY a.valor_12m DESC
             LIMIT 10
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    // ── Estoque & Compras ─────────────────────────────────────────────────────

    pub async fn matriz_abc_xyz(&self) -> Result<Vec<MatrizAbcXyzResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT classe_abc AS abc, COALESCE(classe_xyz, '—') AS xyz,
                   COUNT(*)::bigint AS produtos,
                   COALESCE(SUM(valor_imobilizado), 0)::bigint AS valor_centavos
              FROM bi.analise_produtos
             GROUP BY 1, 2
             ORDER BY 1, 2
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn rupturas(&self) -> Result<Vec<RupturaResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT produto_id, sku, descricao, classe_abc::text AS classe_abc, quantidade,
                   cobertura_dias,
                   GREATEST(CEIL(demanda_dia_90d * 30)::int - quantidade, 1) AS sugestao_compra
              FROM bi.analise_produtos
             WHERE estoque_minimo > 0 AND quantidade <= estoque_minimo AND demanda_dia_90d > 0
             ORDER BY classe_abc, cobertura_dias NULLS FIRST
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn estoque_morto(&self) -> Result<Vec<EstoqueMortoResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT produto_id, sku, descricao, quantidade,
                   valor_imobilizado AS valor_centavos, dias_sem_venda
              FROM bi.analise_produtos
             WHERE quantidade > 0 AND valor_imobilizado > 0
               AND (dias_sem_venda IS NULL OR dias_sem_venda >= 90)
             ORDER BY valor_imobilizado DESC
             LIMIT 10
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Giro por categoria usando o estoque atual como proxy do estoque médio
    /// (o histórico verdadeiro acumula em fato_estoque_snapshot com o tempo).
    pub async fn giro_categorias(&self) -> Result<Vec<CategoriaGiroResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT categoria,
                   SUM(receita_12m)::bigint AS receita_centavos,
                   CASE WHEN SUM(receita_12m) > 0
                        THEN SUM(margem_12m)::float8 / SUM(receita_12m) * 100 END AS margem_percent,
                   SUM(valor_imobilizado)::bigint AS valor_estoque_centavos,
                   CASE WHEN SUM(valor_imobilizado) > 0
                        THEN (SUM(receita_12m) - SUM(margem_12m))::float8 / SUM(valor_imobilizado) END AS giro
              FROM bi.analise_produtos
             GROUP BY categoria
             ORDER BY receita_centavos DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn pedidos_parados(&self) -> Result<Vec<PedidoParadoResult>, AppError> {
        sqlx::query_as(
            r#"
            SELECT p.pedido_id, f.razao_social AS fornecedor, p.total_centavos,
                   p.status::text AS status,
                   EXTRACT(day FROM NOW() - p.criado_em)::int AS dias_parado
              FROM proj_pedidos_compra p
              JOIN proj_fornecedores f
                    ON f.tenant_id = p.tenant_id AND f.fornecedor_id = p.fornecedor_id
             WHERE p.status IN ('gerado', 'aprovado') AND p.criado_em < NOW() - INTERVAL '7 days'
             ORDER BY p.total_centavos DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Retorna `false` quando o alerta não existe (ou já não está aberto).
    pub async fn feedback(&self, alerta_id: Uuid, acao: &str) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE bi.alertas
               SET status = $2,
                   atualizado_em = NOW(),
                   resolvido_em = CASE WHEN $2 = 'resolvido' THEN NOW() ELSE resolvido_em END,
                   snooze_ate   = CASE WHEN $2 = 'ignorado'  THEN NOW() + INTERVAL '30 days' ELSE snooze_ate END
             WHERE alerta_id = $1 AND status IN ('novo', 'visto')
            "#,
        )
        .bind(alerta_id)
        .bind(acao)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(result.rows_affected() > 0)
    }
}
