pub mod catalogo;
pub mod compras;
pub mod crm;
pub mod estoque;
pub mod financeiro;
pub mod fiscal;
pub mod fornecedores;
pub mod identity;
pub mod orcamentos;
pub mod vendas;

/// Faz parse de um Uuid vindo de um evento de domínio.
///
/// Eventos persistidos no event store sempre carregam uuids válidos (gerados
/// pelo próprio agregado), então uma falha aqui indica corrupção/evento
/// inesperado. Loga e retorna `None` em vez de cair para `Uuid::nil()`, para
/// que o chamador pule a atualização da projeção em vez de gravar sob um id
/// incorreto.
pub(crate) fn parse_uuid(field: &str, value: &str) -> Option<uuid::Uuid> {
    uuid::Uuid::parse_str(value)
        .inspect_err(|err| {
            tracing::error!(field, value, error = %err, "projeção: uuid inválido no evento");
        })
        .ok()
}
