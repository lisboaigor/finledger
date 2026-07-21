use pharos_app::{CommandHandler, ValidationError};
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::bi::application::handler::BiHandlers;
use crate::error::AppError;

/// Feedback do gestor sobre um alerta: `resolvido` encerra; `ignorado`
/// silencia a mesma regra+entidade por 30 dias (respeitado pelo recálculo).
#[external_fields]
#[derive(Command, Deserialize)]
pub struct RegistrarFeedbackAlerta {
    #[external]
    pub alerta_id: Uuid,
    pub acao: String,
}

impl CommandHandler<RegistrarFeedbackAlerta> for BiHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: RegistrarFeedbackAlerta) -> Result<(), AppError> {
        if !matches!(cmd.acao.as_str(), "resolvido" | "ignorado") {
            return Err(AppError::Validation(ValidationError::violation(
                "acao",
                "use \"resolvido\" ou \"ignorado\"",
            )));
        }
        let atualizado = self.repo.feedback(cmd.alerta_id, &cmd.acao).await?;
        if atualizado {
            Ok(())
        } else {
            Err(AppError::NotFound)
        }
    }
}
