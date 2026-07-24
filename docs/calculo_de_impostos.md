# Cálculo de Impostos, Análise de Preços e Integração com BI no Finledger

## 1. Contexto Fiscal: Lógica de Cálculo de Impostos

### 1.1 Tipos de Tributos Principais (TributoTipo)

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/fiscal/domain/tributacao/aliquota.rs:8-32`

O sistema reconhece 8 tipos de tributos, todos armazenados em pontos-base (bps = 1/100 de 1%):

| Tipo de Tributo | Código  | Caso de Uso                                        |
| --------------- | ------- | -------------------------------------------------- |
| ICMS            | icms    | Imposto estadual sobre consumo (legado, 2025-2032) |
| ISS             | iss     | Imposto sobre serviços (legado, 2025-2032)         |
| PIS             | pis     | Contribuição social (extinta a partir de 2027)     |
| COFINS          | cofins  | Contribuição social (extinta a partir de 2027)     |
| CBS             | cbs     | Contribuição social (a partir de 2027)             |
| IBS UF          | ibs_uf  | IBS estadual (a partir de 2026)                    |
| IBS Mun         | ibs_mun | IBS municipal (a partir de 2026)                   |
| IS              | is      | Imposto seletivo (a partir de 2027, LC 214/2025)   |

**Objeto de Valor Aliquota (`Aliquota`):**

- Armazenado internamente como `i32` em pontos-base (ex.: 1800 bps = 18%)
- Faixa: 0 a 20.000 bps (0% a 200%), validada em `TryFrom::<i32>`
- Aritmética: arredondamento simétrico "half-up" apenas com inteiros, para evitar erros de ponto flutuante

**Métodos-chave:**

- `aplicar(&self, base_centavos: i64) -> i64`: Aplica a alíquota sobre a base em centavos, com um único arredondamento "half-up"
- `aplicar_reduzida(&self, base_centavos: i64, reducao_bps: i32) -> i64`: Aplica a alíquota reduzida em um único arredondamento (sem truncamento entre etapas)

**Exemplo (linhas 94-100):**

```rust
let a = Aliquota::try_from(1800).expect("18%");
assert_eq!(a.aplicar(10_000), 1_800);  // R$ 100,00 × 18% = R$ 18,00
assert_eq!(Aliquota::try_from(65).expect("bps").aplicar(100), 1);  // 0,65¢ → arredonda para 1
```

### 1.2 Perfil Fiscal (PerfilFiscal)

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/fiscal/domain/tributacao/perfil_fiscal.rs:122-144`

O perfil fiscal do tenant determina como o motor calcula os impostos:

```rust
pub struct PerfilFiscal {
    pub regime: RegimeTributario,  // SimplesNacional / LucroPresumido / LucroReal
    pub uf: Uf,                     // Sigla do estado (2 letras maiúsculas)
    pub codigo_municipio: CodigoMunicipio,  // Código IBGE do município (7 dígitos)
    pub crt: Crt,                   // Campo CRT na NF (1=Simples, 2=Simples+, 3=Normal, 4=MEI)
    pub ibs_cbs_regime_regular: bool,  // Simples optando pelo regime regular de IBS/CBS
    pub aliquota_das_bps: Option<Aliquota>,  // Alíquota efetiva do DAS (para o Simples Nacional)
    pub configurado: bool,           // Configurado pelo usuário vs. padrão legado
}
```

**Três regimes:**

1. **Simples Nacional** (`simples_nacional`): Recolhe um único DAS mensal (sem imposto por venda)
2. **Lucro Presumido** (`lucro_presumido`): Assume margens de lucro e calcula impostos sobre a presunção
3. **Lucro Real** (`lucro_real`): Tributação baseada no lucro real

**Tratamento do MEI (`src/fiscal/application/queries/aliquota_efetiva.rs`):**

- O MEI possui CRT=4 dentro do Simples Nacional
- Ponto-chave: tenants MEI SEM perfil configurado recebem `aliquota_efetiva_bps = 0` retornado para a precificação (pois pagam um DAS fixo, não um percentual)
- O frontend recorre à entrada manual de imposto (padrão 0) — sem presumir 21,65% como em um regime normal
- Quando o MEI configura seu perfil, passa a receber a alíquota efetiva real do seu DAS

**Fallback Legado (`padrao_legado()`):**

- SP, Simples Nacional, CRT 1, sem alíquota de DAS configurada
- Usado para tenants anteriores ao motor — preserva exatamente os valores históricos das NFs
- NÃO aplica as regras do Simples (não destaca CSOSN, destaca ICMS/PIS/COFINS de forma incorreta)
- Caminho correto: o tenant configura o perfil → o tratamento adequado é ativado

### 1.3 Fases de Transição Tributária (FaseTransicao)

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/fiscal/domain/tributacao/fase_transicao.rs`

A reforma tributária brasileira (EC 132/2023, LC 214/2025) se desenrola em fases, cada uma com tributos aplicáveis e fatores de redução distintos:

| Fase           | Anos      | ICMS/ISS        | PIS/COFINS       | CBS/IBS                                      | IS (Seletivo) | Observações                                         |
| -------------- | --------- | --------------- | ---------------- | -------------------------------------------- | ------------- | --------------------------------------------------- |
| Legado         | ≤2025     | 100%            | Sim (65+300 bps) | Não                                          | Não           | Somente o regime atual                              |
| Teste2026      | 2026      | 100%            | Sim              | 0,9% CBS + 0,1% IBS informativo              | Não           | Ano de teste: CBS/IBS exibidos, mas não vinculantes |
| CBS2027_2028   | 2027-28   | 100%            | Não (extinto)    | CBS 8,8% + IBS 0,1%                          | Sim (por NCM) | PIS/COFINS terminam; começa o imposto seletivo      |
| ReducaoIcmsIss | 2029-2032 | Redução gradual | Não              | IBS aumenta gradualmente                     | Sim           | ICMS/ISS caem de 90% para 60% em 4 anos             |
| Plena2033      | 2033+     | 0% (extinto)    | Não              | CBS 8,8% + IBS 17,7% (14,16% UF + 3,54% mun) | Sim           | Somente imposto sobre consumo (CBS/IBS/IS)          |

**Fator de redução gradual (2029-2032):**

- 2029: fator = 9000 bps → ICMS/ISS a 90% da alíquota cheia
- 2030: fator = 8000 bps → 80%
- 2031: fator = 7000 bps → 70%
- 2032: fator = 6000 bps → 60%

**Flags de fase (métodos que retornam `true` se o tributo se aplica):**

- `cobra_legado_estadual()`: ICMS/ISS ainda são cobrados? (`false` somente na fase Plena2033)
- `cobra_pis_cofins()`: PIS/COFINS ainda são cobrados? (`true` apenas em Legado + Teste2026)
- `destaca_ibs_cbs()`: CBS/IBS são destacados? (`false` apenas em Legado)
- `base_ibs_cbs_por_fora()`: CBS/IBS extraídos "por fora" do preço? (`true` a partir de 2027, quando o preço é bruto e a base de cálculo precisa ser calculada de forma retroativa)

### 1.4 Motor de Cálculo de Impostos (MotorTributario)

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/fiscal/domain/tributacao/motor.rs:40-149`

Serviço de domínio puro e testável: dado um contexto (fase + perfil), alíquotas (resolvidas por data/NCM/classe) e a base do item da nota fiscal, calcula todos os valores de imposto.

```rust
pub struct MotorTributario;

impl MotorTributario {
    pub fn calcular_item(
        ctx: &ContextoFiscal,           // fase + perfil fiscal
        aliquotas: &AliquotasItem,      // resolvidas para o produto
        classe: &ClasseTributariaInfo,  // classe tributária (ex.: 000001=integral, 200003=redução de 60%)
        base_centavos: i64,             // total da linha da nota fiscal (preço)
    ) -> ImpostoItem
}
```

**Etapas de cálculo (linhas 46-149):**

1. **Impostos legados (ICMS/ISS):**
   - Se Simples recolhe por dentro: 0 (recolhido dentro do DAS)
   - Caso contrário: alíquota cheia até 2028; redução gradual em 2029-32; extinto a partir de 2033
   - Aplicado com redução de classe (ex.: redução de 60% para a classe 200003)
2. **PIS/COFINS:**
   - Se Simples recolhe por dentro: 0
   - Caso contrário: aplicado somente em 2025-2026; extinto a partir de 2027
3. **CBS/IBS/IS:**
   - Até 2026 (Teste2026): calculado diretamente sobre o preço (informativo)
   - A partir de 2027: o preço é BRUTO (já embute os impostos), então a base é extraída de forma retroativa:
     - Fórmula: `base = preço × 10⁸ / ((10⁴ + IS) × (10⁴ + CBS+IBS))`
     - Garante que: `base + IS(base) + (CBS+IBS)(base+IS) = preço` (exato)
   - A partir de 2027, para o Simples que recolhe por dentro: CBS/IBS informativos (recolhimento via DAS)
   - Imposto Seletivo (IS): apenas a partir de 2027, NÃO é afetado pela redução de classe (LC 214)
   - Erro residual de arredondamento é distribuído ao maior tributo (no máximo alguns centavos)
4. **Custo do DAS (Simples configurado):**
   - Se Simples recolhe por dentro e `aliquota_das_bps` está configurada: DAS = `aliquota_das_bps × base`
   - Usado pela precificação, NÃO destacado na NF (é o custo do tenant)
   - Registra um aviso (log) se for Simples mas sem alíquota de DAS configurada (custo = 0)

**Valor de retorno (`ImpostoItem`, linhas 133-149):**

```rust
pub struct ImpostoItem {
    pub icms_centavos: i64,           // Imposto estadual
    pub iss_centavos: i64,            // Imposto sobre serviços
    pub pis_centavos: i64,
    pub cofins_centavos: i64,
    pub cbs_centavos: i64,            // Novo regime
    pub ibs_uf_centavos: i64,         // Novo regime
    pub ibs_mun_centavos: i64,        // Novo regime
    pub is_centavos: i64,             // Imposto seletivo
    pub c_class_trib: Option<String>, // Classe tributária utilizada
    pub cst_ibs_cbs: Option<String>,  // Código CST para os novos tributos
    pub csosn: Option<String>,        // "102" se Simples (sem crédito)
    pub cst_icms: Option<String>,     // "00" se regime normal
    pub das_centavos: i64,            // Custo do DAS (não aparece na NF)
}
```

### 1.5 Classe Tributária e Redução de Base (ClasseTributaria)

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/fiscal/domain/tributacao/classe_tributaria.rs`

A classe tributária (`cClassTrib`, NT 2025.002) identifica a posição do produto no arcabouço da LC 214/2025:

```rust
pub struct ClasseTributariaInfo {
    pub classe: ClasseTributaria,  // "000001", "200003", "410001", etc. (6 dígitos)
    pub cst_ibs_cbs: String,       // CST para o novo regime ("000", "200", "410")
    pub reducao_bps: i32,          // Redução em bps: 0=integral, 6000=redução de 60%, 10000=zerado
}
```

**Classes padrão (Migração 009, linhas 22-26):**

- `000001`: Tributação integral (CST 000, reducao 0)
- `200003`: Redução de 60% conforme Anexo VII da LC 214 (CST 200, reducao 6000) → CBS efetivo = 8,8% × 40% = 3,52%
- `410001`: Alíquota zero, cesta básica (CST 410, reducao 10000) → CBS = 0

**Aplicação da redução:**

- Somente aos valores de CBS/IBS (durante a fase de cálculo)
- NÃO ao Imposto Seletivo (IS) — permanece integral (LC 214)
- NÃO ao ICMS/ISS legados (a redução é um conceito de 2033+)

### 1.6 Constantes de CFOP e sua Derivação

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/fiscal/domain/cfop.rs:15-28`

O CFOP (Código Fiscal de Operações e Prestações) identifica o tipo de operação na NF:

| CFOP | Operação                                        | Condição               |
| ---- | ----------------------------------------------- | ---------------------- |
| 5102 | Venda de revenda dentro do estado               | Saída, intraestadual   |
| 5405 | Venda de revenda dentro do estado (com ICMS-ST) | Saída + ST             |
| 6102 | Venda de revenda interestadual                  | Saída, interestadual   |
| 6404 | Venda de revenda interestadual (com ICMS-ST)    | Saída + ST             |
| 1202 | Devolução dentro do estado                      | Entrada, intraestadual |
| 2202 | Devolução interestadual                         | Entrada, interestadual |

**Lógica de seleção (linhas 46-62):**

```rust
pub fn resolver_cfop(
    op: TipoOperacao,                  // Venda ou Devolucao
    uf_emitente: &str,                 // Estado do vendedor
    uf_destinatario: Option<&str>,     // Estado do comprador (None = consumidor)
    modelo: &ModeloNF,                 // NFe ou NFCe
    tem_st: bool,                      // Substituição tributária de ICMS? (adiado — issue #16)
) -> &'static str
```

**Notas sobre ICMS-ST:** O cálculo da Substituição Tributária (cálculo de MVA) foi adiado para a issue #16; o CFOP é selecionado, mas os valores ainda não são calculados.

### 1.7 Constantes Nomeadas na Saída da NF

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/fiscal/domain/value_objects.rs:74-76`

```rust
pub const CSOSN_TRIBUTADA_SEM_ST: &str = "102";        // Simples Nacional (sem crédito)
pub const CST_ICMS_TRIBUTADA_INTEGRAL: &str = "00";   // Regime normal (imposto integral)
```

- **CSOSN 102** (Tributada sem ST): usado quando o perfil é Simples Nacional
- **CST 00** (Tributada integralmente): usado quando o perfil é Lucro Presumido/Real
- **Finalidade (`finNFe`):** 1 = venda normal, 4 = devolução
- **TipoNF (`tpNF`):** 1 = saída (venda), 0 = entrada (recebimento/devolução)

---

## 2. Análise de Preços / Precificação: Integração da Alíquota Efetiva

### 2.1 Cálculo da Alíquota Efetiva (aliquota_efetiva_bps)

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/fiscal/application/queries/aliquota_efetiva.rs`

**Consulta:** `GET /api/fiscal/aliquota-efetiva-produtos` (despachada via `query_dispatch(&*s.fiscal, ListarAliquotaEfetivaProdutos)`)

**Propósito:** Fornece o "custo tributário efetivo em percentual (bps)" de cada produto atualmente, na fase de transição vigente e no perfil fiscal do tenant. Usado pelo motor de sugestão de preços para embutir o custo tributário real na margem recomendada.

**Arquitetura (CQRS + Ports & Adapters):** este read segue a convenção do codebase — o query handler (`application/queries/`) é um adapter fino e **não** contém SQL. O acesso ao read model fica no repositório (`infrastructure`); a orquestração de domínio (motor tributário + provider de alíquotas + cálculo dos bps) permanece na aplicação. Concretamente:

- `ListarAliquotaEfetivaProdutos` (`QueryHandler`) em `src/fiscal/application/queries/aliquota_efetiva.rs`
- `PostgresNotaFiscalRepository::listar_produtos_tributaveis()` (o `SELECT` em `proj_produtos`) em `src/fiscal/infrastructure/repository.rs`, retornando `ProdutoTributavel { produto_id, ncm, c_class_trib }`

**Implementação (`QueryHandler::handle`):**

```rust
async fn handle(
    &self,
    _query: ListarAliquotaEfetivaProdutos,
) -> Result<Vec<AliquotaEfetivaProduto>, AppError> {
    const BASE_CENTAVOS: i64 = 1_000_000;  // R$ 10.000 para precisão

    // Regra: sem perfil configurado → retorna vazio (usa entrada manual de imposto, padrão 0)
    let Some(perfil) = self.tenants.obter_perfil_fiscal().await?.para_dominio()? else {
        return Ok(Vec::new());
    };

    let data = hoje_brasil();  // Data de hoje (fuso horário do Brasil)
    let ctx = ContextoFiscal {
        fase: FaseTransicao::de_data(data),
        perfil,
    };
    let informativo = ctx.perfil.ibs_cbs_informativo();  // Simples via DAS?

    // Leitura do read model delegada ao repositório (infrastructure) — o handler não conhece SQL
    let produtos = self.repo.listar_produtos_tributaveis().await?;

    // Cache por (classe, ncm) para evitar reresolver alíquotas por produto
    let mut cache: HashMap<(String, String), i32> = HashMap::new();
    let mut result = Vec::with_capacity(produtos.len());

    for produto in produtos {
        let ProdutoTributavel { produto_id, ncm, c_class_trib: classe_produto } = produto;
        let chave = (classe_produto.clone().unwrap_or_default(), ncm.clone());

        let bps = if let Some(bps) = cache.get(&chave) {
            *bps
        } else {
            // Resolve a classe (ou usa o padrão integral)
            let classe_vo = classe_produto
                .map(ClasseTributaria::try_from)
                .transpose()?;
            let classe = self.aliquotas.classe_info(classe_vo.as_ref()).await?;

            // Resolve as alíquotas para essa data/perfil/classe/NCM
            let aliquotas = self.aliquotas.resolver(data, &ctx.perfil, &classe.classe, &ncm).await?;

            // Calcula os impostos usando o motor puro
            let imposto = MotorTributario::calcular_item(&ctx, &aliquotas, &classe, BASE_CENTAVOS);

            // CRÍTICO: usa custo_vendedor_centavos, não a soma de todos os impostos
            let custo = imposto.custo_vendedor_centavos(informativo);

            // Converte para pontos-base: (custo × 10.000) / base
            let bps = (custo * 10_000 / BASE_CENTAVOS) as i32;
            cache.insert(chave, bps);
            bps
        };

        result.push(AliquotaEfetivaProduto {
            produto_id,
            imposto_efetivo_bps: bps,
        });
    }
    Ok(result)
}
```

**Fórmula-chave:**

```
imposto_efetivo_bps = (custo_vendedor_centavos × 10.000) / 1.000.000 centavos
                     = (custo_vendedor × 10.000) / R$ 10.000
```

**Cálculo do custo do vendedor (`ImpostoItem::custo_vendedor_centavos`, linhas 166-178):**

```rust
pub fn custo_vendedor_centavos(&self, ibs_cbs_informativo: bool) -> i64 {
    // Impostos legados + imposto seletivo + DAS sempre contam
    let legado_e_seletivo = self.icms_centavos + self.iss_centavos
                           + self.pis_centavos + self.cofins_centavos
                           + self.is_centavos + self.das_centavos;

    if ibs_cbs_informativo {
        // Simples Nacional sem opção pelo regime: CBS/IBS recolhidos no DAS
        return legado_e_seletivo;
    } else {
        // Regimes normais (ou Simples com opção pelo regime): CBS/IBS são custos reais do bolso
        return legado_e_seletivo + self.cbs_centavos + self.ibs_uf_centavos + self.ibs_mun_centavos;
    }
}
```

**Exemplo de cálculo (2033, fase Plena, produto com redução de 60%, base de R$ 100):**

| Tributo           | Alíquota | Percentual efetivo | Cálculo                | Resultado |
| ----------------- | -------- | ------------------ | ---------------------- | --------- |
| ICMS              | 18%      | 0%                 | Extinto → 0            | R$ 0,00   |
| CBS               | 8,8%     | 40%                | 8,8% × 40% = 3,52%     | R$ 3,52   |
| IBS UF            | 14%      | 40%                | 14% × 40% = 5,6%       | R$ 5,60   |
| IBS Mun           | 3,5%     | 40%                | 3,5% × 40% = 1,4%      | R$ 1,40   |
| Custo do vendedor | —        | —                  | Soma                   | R$ 10,52  |
| BPS               | —        | —                  | (1,052 × 10.000) / 100 | 1052 bps  |

Ou seja, a precificação embutiria cerca de 10,52% como custo tributário para esse produto.

### 2.2 Endpoints da Rota de Precificação

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/web/routes/catalogo.rs:97-200`

| Endpoint                                   | Método | Propósito                                                               |
| ------------------------------------------ | ------ | ----------------------------------------------------------------------- |
| `/api/catalogo/margens`                    | GET    | Lista as margens por categoria (`margem_bps`)                           |
| `/api/catalogo/margens/{categoria}`        | POST   | Define/atualiza a margem da categoria                                   |
| `/api/catalogo/precificacao-produtos`      | GET    | Lista as sobreposições por produto (margem, custo fixo, frete)          |
| `/api/catalogo/produtos/{id}/precificacao` | POST   | Define sobreposições no nível do produto                                |
| `/api/catalogo/giro-produtos`              | GET    | Giro do produto (unidades em 90 dias, dias sem venda, saldo em estoque) |
| `/api/catalogo/elasticidade/{id}`          | GET    | Elasticidade preço-demanda da última reprecificação                     |

**Entradas de precificação (`ProdutoPrecificacaoResult`, linhas 26-31):**

```rust
pub struct ProdutoPrecificacaoResult {
    pub produto_id: Uuid,
    pub margem_bps: Option<i32>,                        // Margem sobreposta
    pub custo_fixo_unitario_centavos: Option<i64>,      // Custo fixo por unidade
    pub frete_venda_bps: Option<i32>,                   // Frete de venda como %
}
```

**Lógica do frontend (inferida a partir dos dados do repositório):**

```
Preço Sugerido = (Custo + CustoFixo + CustoTributário + Frete) / (1 − Margem − TaxaCartão)
```

Onde:

- **Custo:** custo do produto (cadastro ou médio real)
- **CustoFixo:** despesa fixa por unidade (categoria ou produto)
- **CustoTributário:** `imposto_efetivo_bps` da consulta de alíquota efetiva
- **Frete:** média do fornecedor (`frete_tipico_bps`) ou sobreposição do produto (`frete_venda_bps`)
- **Margem:** margem da categoria ou do produto (`margem_bps`)
- **TaxaCartão:** % das vendas via cartão × taxa da maquininha (`taxa_bps`)

### 2.3 Mix de Pagamento por Cartão

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/src/catalogo/infrastructure/precificacao_repository.rs:300-315`

```rust
pub async fn mix_pagamento(&self) -> Result<MixPagamentoResult, AppError> {
    // Últimos 90 dias, apenas vendas confirmadas
    // % da receita total que veio via cartão de crédito/débito
    sqlx::query_as(
        "SELECT COALESCE(ROUND(10000.0
                    * SUM(total_centavos) FILTER (WHERE forma_pagamento ILIKE 'Cartão%')
                    / NULLIF(SUM(total_centavos), 0)), 10000)::INT AS participacao_cartao_bps,
                COUNT(*)::BIGINT AS amostra_vendas
         FROM proj_vendas
         WHERE tenant_id = $1 AND status = 'confirmada'
           AND confirmada_em >= NOW() - INTERVAL '90 days'"
    )
    .fetch_one(...)
    .await
}
```

**Retorna:**

- `participacao_cartao_bps`: 10000 = 100% no cartão (padrão se houver menos de 5 vendas em 90 dias)
- `amostra_vendas`: contagem para o frontend decidir o nível de confiança

---

## 3. BI / Análise de Preços: Relatórios de Impostos e Alertas

### 3.1 Campos de Impostos nas Tabelas Fato do BI

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/docker/postgres/bi.sql:66-94`

```sql
CREATE TABLE bi.fato_vendas_item (
    tenant_id                 UUID,
    item_id                   UUID,
    venda_id                  UUID,
    produto_id                UUID,
    quantidade                INTEGER,
    receita_centavos          BIGINT,        -- Total da linha de venda (preço × qtd)
    custo_centavos            BIGINT,        -- Custo (preço de compra × qtd)
    margem_centavos           BIGINT,        -- Margem bruta: receita − custo
    impostos_centavos         BIGINT,        -- Custo tributário do vendedor (da NF)
    margem_liquida_centavos   BIGINT,        -- Margem líquida: receita − custo − impostos
    ...
);
```

**Distinção-chave:**

- **Margem bruta** (`margem_centavos`): receita − custo (antes dos impostos)
- **Margem líquida** (`margem_liquida_centavos`): receita − custo − impostos (considerando tributos)

### 3.2 ETL de Reconciliação Tributária: bi.refresh_impostos_vendas()

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/docker/postgres/bi.sql:543-592`

Materializado após cada emissão de NF, recalcula o custo tributário do vendedor por linha de venda.

```sql
CREATE OR REPLACE FUNCTION bi.refresh_impostos_vendas() RETURNS BIGINT AS $$
DECLARE n BIGINT;
BEGIN
    -- Etapa 1: agrega os impostos por produto por venda a partir da projeção de NF
    WITH imp AS (
        SELECT ni.tenant_id, ni.venda_id, ni.produto_id,
               SUM(ni.icms_centavos + ni.iss_centavos + ni.pis_centavos
                   + ni.cofins_centavos + ni.is_centavos
                   -- CRÍTICO: CBS/IBS somente se NÃO for informativo (não Simples via DAS)
                   + CASE WHEN ni.ibs_cbs_informativo THEN 0
                          ELSE ni.cbs_centavos + ni.ibs_uf_centavos + ni.ibs_mun_centavos END
               ) AS total
          FROM proj_nf_itens ni
          JOIN proj_notas_fiscais nf ON nf.tenant_id = ni.tenant_id AND nf.nf_id = ni.nf_id
         WHERE nf.status <> 'cancelada'
         GROUP BY 1, 2, 3
    ),
    -- Etapa 2: mapeia os impostos para a primeira ocorrência do produto na venda (o restante recebe 0)
    alvo AS (
        SELECT f.tenant_id, f.item_id, imp.total,
               ROW_NUMBER() OVER (PARTITION BY f.tenant_id, f.venda_id, f.produto_id
                                  ORDER BY f.item_id) AS rn
          FROM bi.fato_vendas_item f
          JOIN imp ON ...
    )
    -- Etapa 3: atualiza a tabela fato com os impostos e a margem líquida
    UPDATE bi.fato_vendas_item f
       SET impostos_centavos = CASE WHEN a.rn = 1 THEN a.total ELSE 0 END,
           margem_liquida_centavos = f.margem_centavos - CASE WHEN a.rn = 1 THEN a.total ELSE 0 END
      FROM alvo a
     WHERE a.tenant_id = f.tenant_id AND a.item_id = f.item_id;

    -- Etapa 4: limpeza de impostos órfãos (NF cancelada sem substituição)
    UPDATE bi.fato_vendas_item f
       SET impostos_centavos = 0,
           margem_liquida_centavos = f.margem_centavos
     WHERE f.impostos_centavos <> 0
       AND NOT EXISTS (SELECT 1 FROM proj_notas_fiscais nf
                        WHERE nf.tenant_id = f.tenant_id AND nf.venda_id = f.venda_id
                          AND nf.status <> 'cancelada');

    GET DIAGNOSTICS n = ROW_COUNT;
    RETURN n;
END $$;
```

**Lógica-chave:**

- Agrega os impostos por produto (múltiplas linhas do mesmo produto → um total por produto)
- Atribui o total completo à 1ª linha, 0 para as demais (para que a SOMA por produto fique correta)
- Respeita a flag `ibs_cbs_informativo`: se for Simples via DAS, CBS/IBS são excluídos do custo
- Limpa impostos órfãos quando a NF é cancelada (reverte a margem para a bruta)

### 3.3 Agregação de Análise de Produtos em 12 Meses

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/docker/postgres/bi.sql:720-790`

```sql
CREATE TABLE bi.analise_produtos (
    tenant_id            UUID,
    produto_id           UUID,
    receita_12m          BIGINT,        -- Vendas em 12 meses
    margem_12m           BIGINT,        -- Margem bruta em 12 meses
    margem_liquida_12m   BIGINT,        -- Margem LÍQUIDA em 12 meses (com impostos)
    qtd_12m              BIGINT,        -- Unidades vendidas em 12 meses
    cobertura_dias       INTEGER,       -- Cobertura de estoque (dias)
    classe_abc           CHAR(1),       -- Pareto (A=80%, B=15%, C=5%)
    classe_xyz           CHAR(1),       -- Variabilidade da demanda (X=estável, Y=média, Z=errática)
    ...
);
```

**Consulta de agregação (trecho, linhas 720-730):**

```sql
WITH vendas AS (
    SELECT SUM(f.receita_centavos) AS receita,
           SUM(f.custo_centavos) AS custo,
           SUM(f.margem_liquida_centavos) AS margem_liquida,
           ...
      FROM bi.fato_vendas_item f
     WHERE f.tenant_id = ...
       AND f.status = 'confirmada'
       AND f.data_venda >= NOW() - INTERVAL '12 months'
     GROUP BY ...
)
```

### 3.4 Alerta A7: "Produto Vendido Abaixo do Custo"

Arquivo: `/Users/igorlisboa/RustroverProjects/finledger/docker/postgres/bi.sql:999-1019`

Disparado quando o preço de venda de um produto fica abaixo do custo de aquisição (ou do custo real, se rastreado).

```sql
INSERT INTO tmp_alertas
SELECT p.tenant_id, 'A7', p.produto_id::text,
       (GREATEST(p.preco_custo, COALESCE(s.custo_medio, 0)) - p.preco_venda)
           * GREATEST(COALESCE(d.qtd30, 0), 1)  -- Impacto × volume em 30 dias
       0,
       format('"%s" está sendo vendido abaixo do custo', p.descricao),
       format('"%s" (SKU %s) tem preço de venda %s e custo %s — prejuízo de %s por unidade. Corrija o preço no Catálogo.',
              p.descricao, p.sku, bi.fmt_reais(p.preco_venda),
              bi.fmt_reais(GREATEST(p.preco_custo, COALESCE(s.custo_medio, 0))),
              bi.fmt_reais(GREATEST(p.preco_custo, COALESCE(s.custo_medio, 0)) - p.preco_venda)),
       NULL
  FROM proj_produtos p
  LEFT JOIN proj_saldo_estoque s ON s.tenant_id = p.tenant_id AND s.produto_id = p.produto_id
  LEFT JOIN LATERAL (
      SELECT SUM(vi.quantidade) AS qtd30
        FROM proj_vendas_itens vi
        JOIN proj_vendas v ON v.tenant_id = vi.tenant_id AND v.venda_id = vi.venda_id
       WHERE vi.tenant_id = p.tenant_id AND vi.produto_id = p.produto_id
         AND v.status = 'confirmada'
         AND v.confirmada_em >= NOW() - INTERVAL '30 days'
  ) d ON TRUE
 WHERE p.ativo AND p.preco_venda < GREATEST(p.preco_custo, COALESCE(s.custo_medio, 0));
```

**Observação:** o alerta A7 compara o preço de venda com o custo, não com a margem líquida. Perdas na margem bruta disparam esse alerta (o custo tributário é tratado separadamente em `margem_liquida`).

---

## 4. Exemplo de Integração: Fluxo Completo

**Cenário:** tenant Lucro Real (configurado), ano 2033 (fase Plena), vendendo um produto com redução de 60% (classe 200003) a R$ 100.

1. **Alíquota Efetiva (entrada de precificação):**
   - O query handler `ListarAliquotaEfetivaProdutos` lê os produtos via `repo.listar_produtos_tributaveis()` e chama `MotorTributario::calcular_item()` com:
     - Fase: Plena2033
     - Perfil: Lucro Real (`regime_tributario="lucro_real"`, `ibs_cbs_informativo=false`)
     - Classe: 200003 (`reducao_bps=6000`)
     - Base: R$ 10.000 (1.000.000 centavos)
   - O motor retorna: ICMS=0, CBS=3.520¢, IBS_UF=5.600¢, IBS_MUN=1.400¢
   - Custo do vendedor: 3.520 + 5.600 + 1.400 = 10.520 centavos
   - `imposto_efetivo_bps` = (10.520 × 10.000) / 1.000.000 = 1.052 bps (10,52%)

2. **Emissão da nota fiscal:**
   - Venda de 1 unidade a R$ 100 → `ItemNF` com `quantidade=1`, `valor_unitario=10.000¢`
   - O handler chama `MotorTributario::calcular_item()` com a base real=10.000¢
   - Obtém `ImpostoItem` com os impostos proporcionais: CBS=352¢, IBS_UF=560¢, IBS_MUN=140¢
   - Imposto total = 1.052¢
   - A NF inclui: `total_centavos=10.000, cbs_centavos=352, ibs_uf_centavos=560, ibs_mun_centavos=140, c_class_trib="200003", cst_ibs_cbs="200", cst_icms="00"`

3. **Evento e projeção:**
   - Evento `NotaFiscalGerada` é disparado com o `ImpostoItem` congelado
   - A `FiscalProjection` insere em `proj_nf_itens` (tenant_id, nf_id, produto_id, icms_centavos=0, cbs_centavos=352, ibs_uf_centavos=560, ibs_mun_centavos=140, ibs_cbs_informativo=false)

4. **ETL do BI:**
   - `bi.refresh_impostos_vendas()` é disparado
   - Soma os impostos da NF: 0 + 560 + 140 + 352 = 1.052¢
   - Atualiza `bi.fato_vendas_item` com `impostos_centavos=1.052`
   - Calcula `margem_liquida_centavos = margem_centavos − 1.052`

5. **Analytics:**
   - O painel do produto exibe a margem líquida (já deduzidos os impostos)
   - Verificação do alerta A7: o preço de venda (R$ 100) está abaixo do custo? Não → sem alerta
   - A agregação de 12 meses inclui `margem_liquida_12m` (considerando os impostos)

---

## 5. Caso Especial do MEI: Por que a Alíquota Fica Vazia?

**Ponto-chave (`src/fiscal/application/queries/aliquota_efetiva.rs`, guarda `let Some(perfil) = … else { return Ok(Vec::new()) }`):**

O MEI (Simples Nacional, CRT 4) normalmente paga um DAS mensal fixo, não um percentual por venda. O sistema distingue:

1. **MEI + sem perfil configurado** → `imposto_efetivo_bps = []` (lista vazia)
   - O frontend recorre à entrada manual de imposto (padrão 0), pois o imposto proporcional seria enganoso
   - Não se presume 21,65% como em um regime normal
2. **MEI + perfil configurado com `aliquota_das_bps`** → `imposto_efetivo_bps` = calculado
   - Se o tenant define explicitamente a alíquota do DAS (ex.: 4% efetivo), o motor a aplica como custo do vendedor
   - A precificação então embute cerca de 4% como custo tributário, de forma precisa para o seu perfil
3. **Simples (não MEI) sem opção pelo regime** → `ibs_cbs_informativo=true`
   - CBS/IBS são destacados na NF, mas excluídos do custo do vendedor (recolhimento via DAS)
   - A alíquota retornada para a precificação inclui apenas ICMS/ISS (se na fase legada) e o DAS

---

## 6. Resumo de Arquivos-Chave e Números de Linha

| Função/Conceito                          | Arquivo                                             | Linhas         |
| ---------------------------------------- | --------------------------------------------------- | -------------- |
| Aliquota e tipos de tributo              | `src/fiscal/domain/tributacao/aliquota.rs`          | 1-72           |
| Perfil Fiscal                            | `src/fiscal/domain/tributacao/perfil_fiscal.rs`     | 1-244          |
| Fases do FaseTransicao                   | `src/fiscal/domain/tributacao/fase_transicao.rs`    | 1-123          |
| Motor MotorTributario                    | `src/fiscal/domain/tributacao/motor.rs`             | 40-212         |
| Classe tributária (ClasseTributaria)     | `src/fiscal/domain/tributacao/classe_tributaria.rs` | 1-88           |
| ImpostoItem e custo_vendedor             | `src/fiscal/domain/value_objects.rs`                | 82-178         |
| Consulta de alíquota efetiva (query handler) | `src/fiscal/application/queries/aliquota_efetiva.rs` | —          |
| Read model da alíquota efetiva (SQL)     | `src/fiscal/infrastructure/repository.rs` (`listar_produtos_tributaveis`) | — |
| Endpoints de precificação                | `src/web/routes/catalogo.rs`                        | 97-200         |
| Resolução de CFOP                        | `src/fiscal/domain/cfop.rs`                         | 1-128          |
| Agregação de impostos no BI              | `docker/postgres/bi.sql`                            | 67-94, 543-592 |
| Dados de referência da classe tributária | `docker/postgres/migrations/009_ref_tributacao.sql` | 1-87           |
| Projeção de item de NF                   | `docker/postgres/migrations/013_proj_nf_itens.sql`  | 1-40           |
| Mapeamento de evento de NF → projeção    | `src/projections/fiscal.rs`                         | 82-138         |
| Alerta A7 (abaixo do custo)              | `docker/postgres/bi.sql`                            | 999-1019       |

---

## 7. Referência de Fórmulas e Constantes

| Nome                               | Valor                         | Uso                               |
| ---------------------------------- | ----------------------------- | --------------------------------- |
| CSOSN Simples                      | "102"                         | Simples Nacional (sem crédito)    |
| CST Normal                         | "00"                          | Regimes normais                   |
| Limite da alíquota (mín-máx)       | 0-20.000 bps                  | 0%-200%                           |
| Classe tributária padrão           | "000001"                      | Tributação integral (sem redução) |
| Redução de classe: 60%             | 6.000 bps                     | Produtos do Anexo VII da LC 214   |
| Classe com alíquota zero           | 10.000 bps                    | Itens da cesta básica             |
| CBS 2026 (ano de teste)            | 0,9%                          | Informativo (embutido no preço)   |
| IBS 2026                           | 0,1% (0,05 UF + 0,05 Mun)     | Informativo                       |
| CBS a partir de 2027               | 8,8%                          | Estimativa de referência (plena)  |
| IBS a partir de 2033 (referência)  | 17,7% (14,16% UF + 3,54% Mun) | Regime pleno                      |
| Redução gradual do ICMS 2029-2032  | Fatores 90→80→70→60%          | Aplicado via `fator_legado_bps`   |
| Precisão da base (imposto efetivo) | R$ 10.000 (1M centavos)       | Garante a precisão em bps         |

Esta documentação registra a lógica completa de cálculo de impostos, sua integração com a precificação e os relatórios na camada de BI.
