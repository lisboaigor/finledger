# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Finledger is a multi-tenant ERP (auto-parts / workshop domain) with a Rust/Axum backend using Event Sourcing and a Nuxt 4 + PrimeVue frontend. Code, comments, and API routes are in Portuguese.

The backend depends on local path crates from `../pharos-rs` (pharos-core, pharos-macros, pharos-app, pharos-postgres, pharos-axum) — the sibling repo must be present for the build to work.

## Commands

```bash
just dev          # db + backend + frontend in parallel
just db           # Postgres via docker compose (auto-runs docker/postgres/init.sql + seed_demo.sql on first boot)
just back         # cargo run (backend on :3000)
just front        # cd frontend && pnpm dev (Nuxt on :3001)
just test         # cargo test
just stop         # docker compose down

cargo test --test tenant_isolation            # single integration test file
cargo test --test tenant_isolation nome_do_teste  # single test
cargo build --bin finledger
```

Integration tests in `tests/` use `testcontainers` (spin up their own Postgres) — Docker must be running. `tests/helpers.rs` provides `start_postgres`, `setup_db` (replays the DDL portion of `init.sql`), and `in_tenant` (wraps a future in a tenant scope like `require_auth` does).

Backend requires env vars `DATABASE_URL` and `JWT_SECRET` (loaded via `.env`/dotenvy). Clippy lint: `unwrap()`/`expect()` are banned (warn) — use `?` or explicit error handling.

**After changing Rust code, rebuild AND restart the running backend process** — a stale `target/debug/finledger` process will keep serving old behavior.

## Architecture

### Backend (`src/`)

Event-sourced CQRS on top of the pharos crates. Each bounded context (`vendas`, `catalogo`, `crm`, `estoque`, `compras`, `orcamentos`, `fornecedores`, `financeiro`, `fiscal`, `identity`, `tenants`, `backoffice`) follows the same three-layer layout:

- `domain/` — aggregate (raises events via `self.events.raise(...)`), `events.rs`, `value_objects.rs`
- `application/` — `commands.rs` (structs with `#[derive(Command, Deserialize)]`), `handlers.rs`, `queries.rs` (read from `proj_*` projection tables)
- `infrastructure/` — event-store repository

Wiring lives in `src/bootstrap/`: `handlers.rs` builds all command handlers, `events.rs` registers cross-context event subscribers on the `EventBus`, `projections.rs` registers the projection updaters (`src/projections/*` write the `proj_*` read-model tables). Cross-context flows (e.g., confirmed sale → stock decrement → conta a receber → nota fiscal) happen via events on the bus, not direct calls. Returns (`DevolverItensVenda` → `ItensDevolvidos`) follow the same pattern: stock re-enters at current average cost, a total return also cancels the sale and reverses its open conta a receber, and the fiscal side cancels/reissues the NF only when `SEFAZ_INTEGRACAO_ATIVA=true` — otherwise the NF is flagged `cancelamento_pendente` until the SEFAZ integration goes live.

HTTP layer is `src/web/`: `mod.rs` declares every route in one place (public → `require_auth`-protected → `require_backoffice_auth`-protected), handlers live in `src/web/routes/*.rs`.

### BI module (`src/bi/` + `docker/postgres/bi.sql`)

Prescriptive BI: the `bi` Postgres schema (dims/fatos/alertas) plus ETL and the alert engine live in `docker/postgres/bi.sql` as SECURITY DEFINER functions (idempotent — reapply with `psql -U postgres -d finledger -f docker/postgres/bi.sql`; mounted as `zz-bi.sql` in both composes for fresh initdb). `src/bi/job.rs` runs `SELECT bi.executar_etl()` every 5 min (env `BI_ETL_INTERVAL_SECS`); reads follow the CQRS layout (`ObterResumoBi`/`ListarAlertasBi` queries, `RegistrarFeedbackAlerta` command) served at `/bi/*` and consumed by `useBiViewModel` on the dashboard. Facts read from `proj_*` (never `pharos_tenant_aggregates` — it holds only the latest snapshot per aggregate, no history).

### Command deserialization gotcha (recurring 422 source)

Route handlers use the pattern `Json(mut cmd): Json<SomeCommand>` and then overwrite IDs from the URL path (`cmd.venda_id = venda_id`). Any command field populated from the path (or from the JWT, e.g. `vendedor_id`) **must** be marked so serde doesn't require it in the body:

```rust
#[serde(skip_deserializing, default = "Uuid::nil")]
pub venda_id: Uuid,
```

Omitting this causes a 422 because the frontend never sends those fields. Commands whose handlers construct them manually from `Path` params (no `Json` extractor) don't need it.

### Multi-tenancy

- Tenant is resolved from the subdomain: `demo.localhost:3001` → tenant slug `demo`; slugs `admin`/`backoffice` route to the backoffice UI (separate JWT + `require_backoffice_auth`, superadmin permissions as TEXT[], tenant impersonation).
- On the backend, `require_auth` sets a task-local `CURRENT_TENANT` (`pharos_app::TenantContext`); repositories and projections filter by `tenant_id`. Postgres RLS is also defined in `init.sql`.
- Seeds: `bootstrap/seed.rs` + `docker/postgres/seed_demo.sql` create the `demo` tenant (users like `admin`/senha "admin", `carlos.vendedor`/senha "demo") and a superadmin.

### Frontend (`frontend/`)

Nuxt 4 + PrimeVue (Sakai-style layout, Aura theme with the Finledger emerald-teal primary ramp sampled from the logo — see `nuxt.config.ts`) + Tailwind 4. Dev server on :3001 proxies `/api` → backend :3000 (see `nuxt.config.ts` `devProxy`).

- `composables/useApi.ts` / `useAuth.ts` — tenant API+JWT; `useBackofficeApi.ts` / `useBackofficeAuth.ts` — separate backoffice auth; `useSubdomain.ts` — subdomain/tenant detection.
- `middleware/auth.global.ts` guards routes.
- One page per module in `app/pages/`; `terminal.vue` is the PDV (point of sale) and uses `definePageMeta({ layout: false })` — it must include its own `<Toast />` since the default layout (which normally provides it) is bypassed. PrimeVue `useToast()` silently no-ops without a rendered `<Toast />`.
- Access the app via `http://demo.localhost:3001` (subdomain required for tenant resolution).

## Debugging

`.zed/debug.json` has two configs: "Debug Finledger" (CodeLLDB, builds and launches the backend) and "Debug Frontend (Nuxt)" (JavaScript adapter running `pnpm dev`). Zed does not support VS Code-style compound launches — start each config individually.
