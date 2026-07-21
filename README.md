# Finledger

**ERP multi-tenant para negócios em geral** (varejo, serviços, distribuição etc.), construído em Rust/Axum com Event Sourcing (CQRS) no backend e Nuxt 4 no frontend. Código, comentários e rotas de API são em português.

> Licenciado sob [MIT](LICENSE-MIT) ou [Apache 2.0](LICENSE-APACHE), à sua escolha.

## Visão geral

Finledger cobre o fluxo operacional completo de um negócio: cadastro de produtos e clientes, PDV (ponto de venda), orçamentos, compras a fornecedores, controle de estoque, contas a pagar/receber, emissão de nota fiscal eletrônica e um módulo de BI prescritivo com alertas automáticos. Nada no domínio é específico de um ramo — funciona para qualquer negócio que venda produtos e precise de controle financeiro e fiscal.

É **multi-tenant de verdade**: cada cliente (tenant) tem seu próprio subdomínio, dados isolados por Row-Level Security no Postgres, e um backoffice separado para provisionar e administrar tenants.

## Principais features

- **PDV (ponto de venda)** — terminal de venda rápido (`/terminal`), com impressão térmica de cupom, seleção de forma de pagamento e devolução de itens (parcial ou total, com estorno automático de estoque e financeiro).
- **Catálogo de produtos** — SKU, NCM, preço de custo/venda, categorias.
- **CRM** — cadastro de clientes com CPF/CNPJ, bloqueio por inadimplência.
- **Estoque** — saldo por produto, entradas/saídas, ajustes manuais, alerta de estoque mínimo.
- **Compras** — pedido de compra com fluxo de aprovação → envio → recebimento, gerando entrada de estoque e conta a pagar automaticamente.
- **Orçamentos** — criação, desconto, emissão e conversão direta em venda quando o cliente aceita.
- **Fornecedores** — cadastro com prazos de pagamento e contato.
- **Financeiro** — contas a receber e a pagar, com liquidação e estorno.
- **Fiscal** — emissão de NF-e/NFC-e junto à SEFAZ (opcional — desligado por padrão em ambientes sem homologação fiscal).
- **BI prescritivo** — dashboard de KPIs (caixa, margem, giro de estoque, tendência de vendas), Curva ABC×XYZ de produtos, segmentação RFM de clientes, e um motor de alertas com 12 regras (caixa negativo projetado, contas vencidas, ruptura de estoque, NF rejeitada, venda abaixo do custo, etc.), com feedback de usuário e snooze.
- **Backoffice/superadmin** — provisionamento de novos tenants, permissões granulares (TEXT[]), impersonation de tenant para suporte.
- **Multi-tenancy por subdomínio** — `<slug>.dominio` resolve o tenant automaticamente; isolamento reforçado por Row-Level Security no banco (não depende só da aplicação).

## Arquitetura

### Backend (`src/`)

CQRS orientado a eventos sobre os crates [pharos-rs](https://github.com/lisboaigor/pharos-rs) (framework de Event Sourcing próprio: aggregate roots, `CommandHandler`/`QueryHandler`, `EventBus`, repositório Postgres como event store, middleware Axum para propagar o tenant do JWT).

Cada contexto delimitado (bounded context) segue o mesmo layout de três camadas:

```
src/<contexto>/
├── domain/          # aggregate root (raise de eventos), events.rs, value objects
├── application/     # commands.rs, handlers.rs, queries.rs (lê de proj_*)
└── infrastructure/  # repositório sobre o event store
```

Contextos de negócio: `vendas`, `catalogo`, `crm`, `estoque`, `compras`, `orcamentos`, `fornecedores`, `financeiro`, `fiscal`, `identity`, `tenants`, `backoffice`, `bi`.

A junção acontece em `src/bootstrap/`:
- `handlers.rs` monta todos os command handlers;
- `events.rs` registra os subscribers cross-context no `EventBus` — fluxos como venda confirmada → baixa de estoque → conta a receber → nota fiscal acontecem via eventos, nunca por chamada direta entre módulos;
- `projections.rs` registra os updaters que materializam as tabelas de leitura `proj_*` (`src/projections/`).

Camada HTTP em `src/web/`: `mod.rs` declara todas as rotas (públicas → protegidas por `require_auth` → protegidas por `require_backoffice_auth`); handlers em `src/web/routes/*.rs`.

### Módulo de BI (`src/bi/` + `docker/postgres/bi.sql`)

BI prescritivo implementado como schema `bi` no Postgres (dimensões, fatos, alertas) com ETL e motor de alertas escritos em funções `SECURITY DEFINER` idempotentes. `src/bi/job.rs` roda `SELECT bi.executar_etl()` periodicamente; as leituras seguem o padrão CQRS normal (`ObterResumoBi`/`ListarAlertasBi`), servidas em `/bi/*` e consumidas pelo dashboard via `useBiViewModel`. Os fatos são lidos sempre de `proj_*` — nunca da tabela de snapshot `pharos_tenant_aggregates`, que guarda só o estado mais recente de cada agregado, sem histórico.

### Multi-tenancy

- Tenant resolvido pelo subdomínio da requisição (frontend: `useSubdomain`); slugs `admin`/`backoffice` roteiam para a UI de backoffice.
- No backend, `require_auth` extrai o `tenant_id` do JWT e seta um task-local (`pharos_app::TenantContext`); repositórios e projeções filtram por `tenant_id`, e Row-Level Security no Postgres reforça o isolamento no próprio banco — mesmo que a lógica da aplicação falhe.
- Backoffice usa JWT e permissões separadas (superadmin, `TEXT[]` de permissões granulares), com impersonation de tenant para suporte (token de curta duração, marcado para auditoria).

### Frontend (`frontend/`)

Nuxt 4 + Vue 3, [shadcn-vue](https://www.shadcn-vue.com/) (Reka UI) + Tailwind 4, gráficos com `@unovis`, toasts com `vue-sonner`. Uma página por módulo em `app/pages/` (dashboard, PDV, catálogo, clientes, estoque, compras, orçamentos, fornecedores, financeiro, fiscal, análises de BI, administração de usuários/tenants).

Padrão MVVM: cada tela tem um `composables/viewmodels/use*ViewModel.ts` correspondente. Composables principais: `useApi`/`useAuth` (tenant), `useBackofficeApi`/`useBackofficeAuth` (backoffice, autenticação separada), `useSubdomain` (detecção de tenant), `useThermalPrint` (impressão térmica do PDV).

## Rodando localmente

Requer o repositório irmão [`pharos-rs`](https://github.com/lisboaigor/pharos-rs) (dependência Rust) e Docker (para o Postgres e para os testes de integração via `testcontainers`).

```bash
just dev          # sobe banco + backend + frontend em paralelo
just db           # só o Postgres (docker compose)
just back         # cargo run — backend em :3000
just front        # frontend em :3001 (Nuxt)
just test         # cargo test (integração via testcontainers)
just stop         # docker compose down
```

Configure `DATABASE_URL` e `JWT_SECRET` via `.env` (veja `.env.example`). O tenant de demonstração (`demo`) é semeado automaticamente na primeira subida do banco — acesse via `http://demo.localhost:3001`.

## Testes

Testes de integração em `tests/` (um arquivo por contexto, mais `tenant_isolation`, `cross_bc_integration`, `projections_integration`) sobem seu próprio Postgres via `testcontainers` — Docker precisa estar rodando.

```bash
cargo test                                    # suíte completa
cargo test --test tenant_isolation            # um arquivo específico
cargo test --test tenant_isolation nome_teste # um teste específico
```

Clippy é usado como lint: `unwrap()`/`expect()` são banidos (warn) em favor de tratamento de erro explícito com `?`.

## Deploy

`docker-compose.prod.yml` sobe a aplicação junto com Prometheus e Grafana para observabilidade. `scripts/deploy.py` automatiza deploy para uma VPS via `git archive` + rsync, com migrações idempotentes e health check pós-deploy. Veja `.env.prod.example` e `scripts/.deploy.env.example` para as variáveis necessárias.

## Licença

Este projeto é distribuído sob os termos das licenças [MIT](LICENSE-MIT) e [Apache 2.0](LICENSE-APACHE), à sua escolha.
