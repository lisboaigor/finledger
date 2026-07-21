use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::bi::application::handler::BiHandlers;
use crate::error::AppError;

#[derive(Serialize, sqlx::FromRow)]
pub struct AlertaResult {
    pub alerta_id: Uuid,
    pub codigo: String,
    pub titulo: String,
    pub mensagem: String,
    pub link: String,
    pub impacto_centavos: i64,
    pub urgencia_dias: i32,
    pub score: f64,
    pub status: String,
}

#[derive(Query)]
#[query(result = Vec<AlertaResult>)]
pub struct ListarAlertasBi {
    #[trace(display)]
    pub limite: i64,
}

impl QueryHandler<ListarAlertasBi> for BiHandlers {
    type Error = AppError;

    async fn handle(&self, q: ListarAlertasBi) -> Result<Vec<AlertaResult>, AppError> {
        self.repo.alertas(q.limite.clamp(1, 50)).await
    }
}
