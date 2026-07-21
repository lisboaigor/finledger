//! BI prescritivo: KPIs do dashboard "Hoje" e motor de alertas.
//!
//! Segue o mesmo layout CQRS dos demais contextos: queries/commands despachados
//! via `query_dispatch`/`dispatch` para `BiHandlers`, com o acesso a dados em
//! `infrastructure::repository`. O warehouse vive no schema `bi` (ver
//! `docker/postgres/bi.sql`); o ETL e o recálculo de alertas são funções
//! SECURITY DEFINER no banco, agendadas pelo `job`.

pub mod application;
pub mod infrastructure;
pub mod job;
pub mod outbox;
