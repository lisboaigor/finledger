use pharos_postgres::Pool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::fiscal::domain::tributacao::{
    Aliquota, CodigoMunicipio, Crt, PerfilFiscal, RegimeTributario, Uf,
};
use crate::shared::tenant::current_tenant_id;

#[derive(Serialize, sqlx::FromRow, Clone)]
pub struct TenantResult {
    pub tenant_id: Uuid,
    pub slug: String,
    pub nome: String,
    pub status: String,
    pub plano: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct CustoFixoResult {
    pub nome: String,
    pub valor_centavos: i64,
}

pub struct TenantRepository {
    pool: Pool,
}

impl TenantRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Cria um tenant. Em caso de conflito de slug, retorna o tenant_id existente.
    pub async fn criar(&self, slug: &str, nome: &str) -> Result<Uuid, AppError> {
        let row: (Uuid,) = sqlx::query_as(
            "INSERT INTO tenants (slug, nome)
             VALUES ($1, $2)
             ON CONFLICT (slug) DO UPDATE SET slug = EXCLUDED.slug
             RETURNING tenant_id",
        )
        .bind(slug)
        .bind(nome)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(row.0)
    }

    /// Creates a tenant, failing with a validation error when the slug is taken.
    /// Used by backoffice provisioning, where silently reusing an existing tenant
    /// (as `criar` does) would be dangerous — e.g. the compensating delete after a
    /// failed provisioning must never remove a pre-existing tenant.
    pub async fn create_strict(&self, slug: &str, nome: &str) -> Result<Uuid, AppError> {
        let result: Result<(Uuid,), sqlx::Error> =
            sqlx::query_as("INSERT INTO tenants (slug, nome) VALUES ($1, $2) RETURNING tenant_id")
                .bind(slug)
                .bind(nome)
                .fetch_one(&self.pool)
                .await;

        match result {
            Ok((id,)) => Ok(id),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(AppError::Domain(
                pharos_core::DomainError::Validation(format!("slug '{slug}' já está em uso")),
            )),
            Err(e) => Err(AppError::infra(e)),
        }
    }

    /// Removes a tenant row. Only used to compensate a failed provisioning —
    /// a tenant with real data has FK references and the delete will fail.
    pub async fn delete(&self, tenant_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::infra)?;
        Ok(())
    }

    pub async fn buscar_por_slug(&self, slug: &str) -> Result<Option<TenantResult>, AppError> {
        sqlx::query_as("SELECT tenant_id, slug, nome, status, plano FROM tenants WHERE slug = $1")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::infra)
    }

    pub async fn listar(&self) -> Result<Vec<TenantResult>, AppError> {
        sqlx::query_as(
            "SELECT tenant_id, slug, nome, status, plano FROM tenants ORDER BY criado_em",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Suspende um tenant ativo. Falha com `NotFound` se o tenant não existe e
    /// com `DomainError::BusinessRule` se já estiver suspenso — mesma guarda
    /// de transição de estado usada em `identity::Usuario::desativar`, só que
    /// aqui via SQL condicional já que `tenants` não tem agregado de domínio.
    pub async fn suspender(&self, tenant_id: Uuid) -> Result<(), AppError> {
        self.transicionar_status(tenant_id, "suspenso", "já está suspenso")
            .await
    }

    pub async fn buscar_por_id(&self, tenant_id: Uuid) -> Result<Option<TenantResult>, AppError> {
        sqlx::query_as(
            "SELECT tenant_id, slug, nome, status, plano FROM tenants WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn atualizar_nome(&self, tenant_id: Uuid, nome: &str) -> Result<(), AppError> {
        let n = sqlx::query("UPDATE tenants SET nome = $1 WHERE tenant_id = $2")
            .bind(nome)
            .bind(tenant_id)
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

    pub async fn alterar_plano(&self, tenant_id: Uuid, plano: &str) -> Result<(), AppError> {
        let n = sqlx::query("UPDATE tenants SET plano = $1 WHERE tenant_id = $2")
            .bind(plano)
            .bind(tenant_id)
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

    /// Reativa um tenant suspenso. Falha com `NotFound` se o tenant não existe
    /// e com `DomainError::BusinessRule` se já estiver ativo.
    pub async fn reativar(&self, tenant_id: Uuid) -> Result<(), AppError> {
        self.transicionar_status(tenant_id, "ativo", "já está ativo")
            .await
    }

    /// Move `tenants.status` para `novo_status`, mas só quando ele ainda não
    /// está lá — evita que suspender/reativar sejam idempotentes em silêncio,
    /// o que mascararia um duplo-clique ou uma corrida entre dois operadores.
    async fn transicionar_status(
        &self,
        tenant_id: Uuid,
        novo_status: &str,
        mensagem_se_ja_no_estado: &str,
    ) -> Result<(), AppError> {
        let affected = sqlx::query(
            "UPDATE tenants SET status = $1 WHERE tenant_id = $2 AND status != $1",
        )
        .bind(novo_status)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?
        .rows_affected();

        if affected == 1 {
            return Ok(());
        }

        match self.buscar_por_id(tenant_id).await? {
            None => Err(AppError::NotFound),
            Some(_) => Err(AppError::Domain(pharos_core::DomainError::BusinessRule(
                format!("tenant {mensagem_se_ja_no_estado}"),
            ))),
        }
    }

    // ── Configurações self-service ────────────────────────────────────────────
    // Ao contrário dos métodos acima (usados pelo backoffice, cross-tenant, com
    // tenant_id explícito), estes operam sobre o tenant da REQUISIÇÃO ATUAL
    // (CURRENT_TENANT) — só fazem sentido dentro de uma rota autenticada de
    // tenant, nunca em contexto de backoffice.

    /// Feature flag: permite adicionar itens a orçamentos acima do saldo em
    /// estoque. Ausência de linha em `tenants` (não deveria acontecer em
    /// produção) é tratada como permitido, preservando o comportamento
    /// histórico do sistema.
    pub async fn permite_orcamento_sem_estoque(&self) -> Result<bool, AppError> {
        let tenant_id = current_tenant_id()?;
        let permite: Option<bool> = sqlx::query_scalar(
            "SELECT permite_orcamento_sem_estoque FROM tenants WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(permite.unwrap_or(true))
    }

    pub async fn atualizar_configuracoes(
        &self,
        permite_orcamento_sem_estoque: bool,
    ) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query(
            "UPDATE tenants SET permite_orcamento_sem_estoque = $1 WHERE tenant_id = $2",
        )
        .bind(permite_orcamento_sem_estoque)
        .bind(tenant_id)
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

    /// Prazo (em dias) para arquivar vendas/orçamentos não concretizados;
    /// NULL = lixeira automática desligada.
    pub async fn obter_arquivamento_dias(&self) -> Result<Option<i32>, AppError> {
        let tenant_id = current_tenant_id()?;
        let dias: Option<Option<i32>> =
            sqlx::query_scalar("SELECT arquivamento_dias FROM tenants WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::infra)?;
        Ok(dias.flatten())
    }

    pub async fn atualizar_arquivamento_dias(&self, dias: Option<i32>) -> Result<(), AppError> {
        if let Some(d) = dias
            && d < 1
        {
            return Err(pharos_core::DomainError::Validation(
                "O prazo de arquivamento deve ser de pelo menos 1 dia".into(),
            )
            .into());
        }
        let tenant_id = current_tenant_id()?;
        sqlx::query("UPDATE tenants SET arquivamento_dias = $1 WHERE tenant_id = $2")
            .bind(dias)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::infra)?;
        Ok(())
    }

    /// Dados da empresa exibidos nas impressões de venda/orçamento (recibo
    /// térmico). Todos os campos são opcionais — a ausência apenas omite a
    /// linha correspondente no recibo, não é erro.
    pub async fn obter_dados_empresa(&self) -> Result<DadosEmpresa, AppError> {
        let tenant_id = current_tenant_id()?;
        let row = sqlx::query_as(
            "SELECT cnpj, telefone, endereco, chave_pix, informacoes_adicionais
             FROM tenants WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(row.unwrap_or_default())
    }

    pub async fn atualizar_dados_empresa(&self, dados: DadosEmpresa) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query(
            "UPDATE tenants
             SET cnpj = $1, telefone = $2, endereco = $3, chave_pix = $4, informacoes_adicionais = $5
             WHERE tenant_id = $6",
        )
        .bind(dados.cnpj)
        .bind(dados.telefone)
        .bind(dados.endereco)
        .bind(dados.chave_pix)
        .bind(dados.informacoes_adicionais)
        .bind(tenant_id)
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

    /// Perfil fiscal do tenant atual (regime tributário, UF/município, CRT).
    /// `regime_tributario` NULL = perfil não configurado → o motor tributário
    /// usa o fallback legado (Simples Nacional/SP).
    pub async fn obter_perfil_fiscal(&self) -> Result<PerfilFiscalDto, AppError> {
        let tenant_id = current_tenant_id()?;
        let row = sqlx::query_as(
            "SELECT regime_tributario, uf, codigo_municipio, crt, ibs_cbs_regime_regular,
                    aliquota_das_bps
             FROM tenants WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(row.unwrap_or_default())
    }

    pub async fn atualizar_perfil_fiscal(&self, dto: PerfilFiscalDto) -> Result<(), AppError> {
        // Valida via o value object do domínio antes de persistir: UF/município/
        // CRT malformados nunca chegam ao banco (o CHECK é só a última linha).
        dto.para_dominio()?;
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query(
            "UPDATE tenants
             SET regime_tributario = $1, uf = $2, codigo_municipio = $3, crt = $4,
                 ibs_cbs_regime_regular = $5, aliquota_das_bps = $6
             WHERE tenant_id = $7",
        )
        .bind(dto.regime_tributario)
        .bind(dto.uf)
        .bind(dto.codigo_municipio)
        .bind(dto.crt)
        .bind(dto.ibs_cbs_regime_regular)
        .bind(dto.aliquota_das_bps)
        .bind(tenant_id)
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

    /// Marca whitelabel do tenant atual (self-service).
    pub async fn obter_marca(&self) -> Result<Marca, AppError> {
        let tenant_id = current_tenant_id()?;
        let row = sqlx::query_as(
            "SELECT marca_nome, marca_logo_data_uri, marca_cor_primaria, marca_fonte,
                    marca_fonte_tamanho, marca_fonte_cor
             FROM tenants WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(row.unwrap_or_default())
    }

    /// Marca de um tenant identificado pelo slug — sem contexto de tenant, para
    /// brandizar o login/landing do subdomínio antes da autenticação. Retorna
    /// `None` quando o slug não existe ou o tenant não está ativo (não vaza
    /// mais do que a própria existência do subdomínio, já pública).
    pub async fn obter_marca_por_slug(&self, slug: &str) -> Result<Option<Marca>, AppError> {
        sqlx::query_as(
            "SELECT marca_nome, marca_logo_data_uri, marca_cor_primaria, marca_fonte,
                    marca_fonte_tamanho, marca_fonte_cor
             FROM tenants WHERE slug = $1 AND status = 'ativo'",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn atualizar_marca(&self, marca: Marca) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query(
            "UPDATE tenants
             SET marca_nome = $1, marca_logo_data_uri = $2,
                 marca_cor_primaria = $3, marca_fonte = $4,
                 marca_fonte_tamanho = $5, marca_fonte_cor = $6
             WHERE tenant_id = $7",
        )
        .bind(marca.marca_nome)
        .bind(marca.marca_logo_data_uri)
        .bind(marca.marca_cor_primaria)
        .bind(marca.marca_fonte)
        .bind(marca.marca_fonte_tamanho)
        .bind(marca.marca_fonte_cor)
        .bind(tenant_id)
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

    /// Configuração de precificação assistida (percentuais em basis points e
    /// custos fixos mensais). Tudo opcional — sem margem configurada, o
    /// frontend simplesmente não mostra sugestão de preço.
    pub async fn obter_config_precificacao(&self) -> Result<ConfigPrecificacao, AppError> {
        let tenant_id = current_tenant_id()?;
        let row = sqlx::query_as(
            "SELECT margem_padrao_bps, imposto_venda_bps, comissao_venda_bps, taxa_cartao_bps,
                    frete_venda_bps, outras_despesas_venda_bps,
                    custos_fixos_mensais_centavos, vendas_mensais_estimadas,
                    faturamento_mensal_estimado_centavos, meta_faturamento_mensal_centavos
             FROM tenants WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(row.unwrap_or_default())
    }

    // ── Custos fixos discriminados ───────────────────────────────────────────
    // O tenant pode detalhar os custos fixos item a item (aluguel, salário,
    // DAS…). Com itens cadastrados, tenants.custos_fixos_mensais_centavos passa
    // a ser a soma deles (mantida aqui); sem itens, o total continua editável
    // livremente na tela de Configurações.

    pub async fn listar_custos_fixos(&self) -> Result<Vec<CustoFixoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT nome, valor_centavos FROM custos_fixos
             WHERE tenant_id = $1 ORDER BY valor_centavos DESC, nome",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn definir_custo_fixo(&self, nome: &str, valor_centavos: i64) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query(
            "INSERT INTO custos_fixos (tenant_id, nome, valor_centavos) VALUES ($1, $2, $3)
             ON CONFLICT (tenant_id, nome) DO UPDATE SET valor_centavos = EXCLUDED.valor_centavos",
        )
        .bind(tenant_id)
        .bind(nome)
        .bind(valor_centavos)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?;
        self.sincronizar_total_custos_fixos(tenant_id).await
    }

    pub async fn remover_custo_fixo(&self, nome: &str) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query("DELETE FROM custos_fixos WHERE tenant_id = $1 AND nome = $2")
            .bind(tenant_id)
            .bind(nome)
            .execute(&self.pool)
            .await
            .map_err(AppError::infra)?
            .rows_affected();
        if n == 0 {
            return Err(AppError::NotFound);
        }
        self.sincronizar_total_custos_fixos(tenant_id).await
    }

    /// Total = soma dos itens; ao remover o último item o total vira NULL
    /// (volta ao modo "não configurado"/edição livre do campo único).
    async fn sincronizar_total_custos_fixos(&self, tenant_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tenants
             SET custos_fixos_mensais_centavos =
                 (SELECT SUM(valor_centavos) FROM custos_fixos WHERE tenant_id = $1)
             WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(())
    }

    pub async fn atualizar_config_precificacao(
        &self,
        mut cfg: ConfigPrecificacao,
    ) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        // Com custos fixos discriminados, a soma dos itens prevalece sobre o
        // total enviado (o campo único fica desabilitado na tela; isto protege
        // contra clients desatualizados sobrescreverem a soma).
        let soma_itens: Option<i64> = sqlx::query_scalar(
            // SUM(BIGINT) devolve NUMERIC no Postgres — o cast mantém o decode em i64.
            "SELECT SUM(valor_centavos)::BIGINT FROM custos_fixos WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::infra)?;
        if soma_itens.is_some() {
            cfg.custos_fixos_mensais_centavos = soma_itens;
        }
        let n = sqlx::query(
            "UPDATE tenants
             SET margem_padrao_bps = $1, imposto_venda_bps = $2, comissao_venda_bps = $3,
                 taxa_cartao_bps = $4, frete_venda_bps = $5, outras_despesas_venda_bps = $6,
                 custos_fixos_mensais_centavos = $7, vendas_mensais_estimadas = $8,
                 faturamento_mensal_estimado_centavos = $10,
                 meta_faturamento_mensal_centavos = $11
             WHERE tenant_id = $9",
        )
        .bind(cfg.margem_padrao_bps)
        .bind(cfg.imposto_venda_bps)
        .bind(cfg.comissao_venda_bps)
        .bind(cfg.taxa_cartao_bps)
        .bind(cfg.frete_venda_bps)
        .bind(cfg.outras_despesas_venda_bps)
        .bind(cfg.custos_fixos_mensais_centavos)
        .bind(cfg.vendas_mensais_estimadas)
        .bind(tenant_id)
        .bind(cfg.faturamento_mensal_estimado_centavos)
        .bind(cfg.meta_faturamento_mensal_centavos)
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
}

/// Percentuais em basis points (1 = 0,01%; 4000 = 40%) — inteiro em vez de
/// float, mesma lógica dos centavos para dinheiro.
#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct ConfigPrecificacao {
    #[serde(default)]
    pub margem_padrao_bps: Option<i32>,
    #[serde(default)]
    pub imposto_venda_bps: Option<i32>,
    #[serde(default)]
    pub comissao_venda_bps: Option<i32>,
    #[serde(default)]
    pub taxa_cartao_bps: Option<i32>,
    #[serde(default)]
    pub frete_venda_bps: Option<i32>,
    #[serde(default)]
    pub outras_despesas_venda_bps: Option<i32>,
    #[serde(default)]
    pub custos_fixos_mensais_centavos: Option<i64>,
    #[serde(default)]
    pub vendas_mensais_estimadas: Option<i32>,
    /// Denominador do rateio proporcional: custos fixos ÷ faturamento viram
    /// um percentual do preço de cada venda.
    #[serde(default)]
    pub faturamento_mensal_estimado_centavos: Option<i64>,
    /// Alvo de crescimento (progresso no dashboard + componente do score);
    /// não entra na fórmula de preço.
    #[serde(default)]
    pub meta_faturamento_mensal_centavos: Option<i64>,
}

/// Perfil fiscal na borda do repositório: colunas cruas de `tenants`.
/// A validação de invariantes fica no domínio (`para_dominio`) — o DTO existe
/// para o round-trip com o frontend, onde tudo é opcional até configurar.
#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct PerfilFiscalDto {
    #[serde(default)]
    pub regime_tributario: Option<String>,
    #[serde(default)]
    pub uf: Option<String>,
    #[serde(default)]
    pub codigo_municipio: Option<String>,
    #[serde(default)]
    pub crt: Option<i16>,
    #[serde(default)]
    pub ibs_cbs_regime_regular: bool,
    /// Alíquota efetiva do DAS em bps (Simples Nacional) — custo tributário do
    /// vendedor quando o Simples está configurado. Opcional/aditivo.
    #[serde(default)]
    pub aliquota_das_bps: Option<i32>,
}

impl PerfilFiscalDto {
    /// `Ok(None)` = perfil não configurado (regime ausente) → chamador usa
    /// `PerfilFiscal::padrao_legado()`. Configurado pela metade é erro de
    /// validação, não fallback silencioso.
    pub fn para_dominio(&self) -> Result<Option<PerfilFiscal>, AppError> {
        let Some(regime) = self.regime_tributario.as_deref() else {
            return Ok(None);
        };
        let regime = RegimeTributario::try_from(regime).map_err(AppError::Domain)?;
        let (Some(uf), Some(municipio), Some(crt)) =
            (self.uf.clone(), self.codigo_municipio.clone(), self.crt)
        else {
            return Err(AppError::Domain(pharos_core::DomainError::Validation(
                "Perfil fiscal incompleto: UF, município e CRT são obrigatórios".into(),
            )));
        };
        Ok(Some(PerfilFiscal {
            regime,
            uf: Uf::try_from(uf).map_err(AppError::Domain)?,
            codigo_municipio: CodigoMunicipio::try_from(municipio).map_err(AppError::Domain)?,
            crt: Crt::try_from(u8::try_from(crt).map_err(|_| {
                AppError::Domain(pharos_core::DomainError::Validation("CRT inválido".into()))
            })?)
            .map_err(AppError::Domain)?,
            ibs_cbs_regime_regular: self.ibs_cbs_regime_regular,
            aliquota_das_bps: self
                .aliquota_das_bps
                .map(Aliquota::try_from)
                .transpose()
                .map_err(AppError::Domain)?,
            // Chegou aqui = veio de configuração explícita do tenant (o
            // fallback `padrao_legado` nasce com `configurado = false`).
            configurado: true,
        }))
    }
}

#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct DadosEmpresa {
    #[serde(default)]
    pub cnpj: Option<String>,
    #[serde(default)]
    pub telefone: Option<String>,
    #[serde(default)]
    pub endereco: Option<String>,
    #[serde(default)]
    pub chave_pix: Option<String>,
    #[serde(default)]
    pub informacoes_adicionais: Option<String>,
}

/// Identidade visual whitelabel do tenant (self-service). Tudo opcional —
/// campos nulos caem no tema/marca padrão (Finledger). O logo vem/vai como
/// data URI (base64) para dispensar object storage. Servida também sem
/// autenticação (por slug) para brandizar o login antes de o usuário entrar.
#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Marca {
    /// Nome exibido no lugar de "Finledger" (topbar, sidebar, login).
    #[serde(default)]
    pub marca_nome: Option<String>,
    /// Logo em data URI (`data:image/...;base64,...`).
    #[serde(default)]
    pub marca_logo_data_uri: Option<String>,
    /// Cor de destaque (accent/primária), hex `#RRGGBB`.
    #[serde(default)]
    pub marca_cor_primaria: Option<String>,
    /// Chave da fonte do wordmark (ex.: `pacifico`); o frontend mapeia para a
    /// font-family e carrega a fonte. Nula → fonte padrão (Grand Hotel).
    #[serde(default)]
    pub marca_fonte: Option<String>,
    /// Tamanho do wordmark em percentual (50..200) sobre o tamanho base de cada
    /// local. Nulo → 100% (padrão).
    #[serde(default)]
    pub marca_fonte_tamanho: Option<i16>,
    /// Cor do texto do wordmark, hex `#RRGGBB`. Nula → cor de texto herdada.
    #[serde(default)]
    pub marca_fonte_cor: Option<String>,
}
