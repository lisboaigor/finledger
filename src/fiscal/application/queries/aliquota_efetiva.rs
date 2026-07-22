use serde::Serialize;
use uuid::Uuid;

/// Alíquota efetiva de imposto (em basis points) que é CUSTO do vendedor para
/// um produto, na fase tributária vigente hoje e no perfil fiscal do tenant.
/// Consumida pela precificação assistida no lugar do imposto manual único —
/// reflete a reforma automaticamente conforme as fases avançam (LC 214/2025).
#[derive(Debug, Clone, Serialize)]
pub struct AliquotaEfetivaProduto {
    pub produto_id: Uuid,
    pub imposto_efetivo_bps: i32,
}
