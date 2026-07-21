use pharos_core::{DomainError, DomainResult};

/// Disponibilidade de um produto para ser adicionado a uma venda/orçamento.
///
/// Resolvida pela camada de aplicação (leitura de `proj_produtos`/
/// `proj_saldo_estoque` — ver `estoque::application::disponibilidade`) e
/// consumida pelas regras de domínio de `Venda`/`Orcamento` ao adicionar
/// itens: a checagem em si (comparar quantidade pretendida × saldo) é regra
/// de negócio e vive em `validar`, não na camada de aplicação.
#[derive(Debug, Clone, Copy)]
pub enum Disponibilidade {
    /// Produto não controla estoque (serviço, mão de obra) — sempre permite.
    NaoControlada,
    /// Produto controla estoque; saldo atual disponível em unidades.
    Controlada(u32),
    /// Confirmação explícita do vendedor para vender/orçar acima do saldo
    /// (venda sob encomenda, ou orçamento com a feature flag do tenant
    /// habilitada) — ignora o saldo.
    SemChecagem,
}

impl Disponibilidade {
    /// Valida se é possível adicionar `quantidade_adicional` unidades, dado
    /// que `ja_no_documento` unidades do mesmo produto já estão no documento
    /// (soma de adições anteriores do mesmo produto_id).
    pub fn validar(&self, ja_no_documento: u32, quantidade_adicional: u32) -> DomainResult<()> {
        let Disponibilidade::Controlada(saldo) = self else {
            return Ok(());
        };
        let total = ja_no_documento.saturating_add(quantidade_adicional);
        if total > *saldo {
            return Err(DomainError::BusinessRule(format!(
                "Estoque insuficiente para este produto: disponível {saldo}, solicitado {total}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controlada_com_saldo_suficiente_permite() {
        assert!(Disponibilidade::Controlada(10).validar(0, 5).is_ok());
    }

    #[test]
    fn controlada_com_saldo_insuficiente_bloqueia() {
        assert!(matches!(
            Disponibilidade::Controlada(3).validar(0, 5),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn controlada_soma_o_que_ja_esta_no_documento() {
        // 2 já adicionadas + 2 novas = 4, saldo 3 → deve bloquear mesmo que
        // a quantidade desta chamada isolada (2) caiba no saldo.
        assert!(matches!(
            Disponibilidade::Controlada(3).validar(2, 2),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn nao_controlada_sempre_permite() {
        assert!(Disponibilidade::NaoControlada.validar(0, 1_000_000).is_ok());
    }

    #[test]
    fn sem_checagem_sempre_permite() {
        assert!(Disponibilidade::SemChecagem.validar(0, 1_000_000).is_ok());
    }
}
