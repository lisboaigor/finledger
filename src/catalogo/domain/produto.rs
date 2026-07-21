use chrono::Utc;
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};

use super::events::CatalogoEvent;
use crate::fiscal::domain::tributacao::ClasseTributaria;
use crate::shared::{Dinheiro, Ncm, NomeNaoVazio, Sku, Unidade};

id_type!(ProdutoId);

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct Produto {
    #[id]
    id: ProdutoId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<CatalogoEvent>,
    sku: Sku,
    descricao: NomeNaoVazio,
    ncm: Ncm,
    unidade: Unidade,
    preco_custo: Dinheiro,
    preco_venda: Dinheiro,
    categoria: String,
    #[serde(default)]
    marca: Option<String>,
    ativo: bool,
    /// FALSE para serviços/itens sem saldo de estoque físico — produtos assim
    /// ficam de fora da checagem de disponibilidade em vendas/orçamentos.
    #[serde(default = "default_true")]
    controla_estoque: bool,
    /// Classe tributária (cClassTrib) da reforma; `None` = tributação
    /// integral (classe padrão) — produtos existentes não precisam de
    /// reclassificação.
    #[serde(default)]
    classe_trib: Option<ClasseTributaria>,
}

fn default_true() -> bool {
    true
}

impl Produto {
    // Getters
    pub fn id(&self) -> &ProdutoId {
        &self.id
    }

    pub fn sku(&self) -> &Sku {
        &self.sku
    }

    pub fn descricao(&self) -> &NomeNaoVazio {
        &self.descricao
    }

    pub fn ncm(&self) -> &Ncm {
        &self.ncm
    }

    pub fn unidade(&self) -> &Unidade {
        &self.unidade
    }

    pub fn preco_custo(&self) -> &Dinheiro {
        &self.preco_custo
    }

    pub fn preco_venda(&self) -> &Dinheiro {
        &self.preco_venda
    }

    pub fn categoria(&self) -> &str {
        &self.categoria
    }

    pub fn marca(&self) -> Option<&str> {
        self.marca.as_deref()
    }

    pub fn ativo(&self) -> bool {
        self.ativo
    }

    pub fn controla_estoque(&self) -> bool {
        self.controla_estoque
    }

    pub fn classe_trib(&self) -> Option<&ClasseTributaria> {
        self.classe_trib.as_ref()
    }

    #[allow(clippy::too_many_arguments)] // flat args mirror the command payload
    pub fn cadastrar(
        sku: String,
        descricao: String,
        ncm: String,
        unidade: String,
        preco_custo: Dinheiro,
        preco_venda: Dinheiro,
        categoria: String,
        marca: Option<String>,
        controla_estoque: bool,
        classe_trib: Option<String>,
    ) -> DomainResult<Self> {
        let sku = Sku::try_from(sku)?;
        let descricao = NomeNaoVazio::try_from(descricao)?;
        let ncm = Ncm::try_from(ncm)?;
        let unidade = Unidade::try_from(unidade)?;
        let classe_trib = classe_trib.map(ClasseTributaria::try_from).transpose()?;

        if preco_venda.centavos() <= 0 {
            return Err(DomainError::Validation(
                "Preço de venda deve ser positivo".into(),
            ));
        }

        if preco_custo.centavos() < 0 {
            return Err(DomainError::Validation(
                "Preço de custo não pode ser negativo".into(),
            ));
        }

        let id = ProdutoId::new();

        let mut events = AggregateEvents::default();

        events.raise(CatalogoEvent::ProdutoCadastrado {
            produto_id: id.to_string(),
            sku: sku.to_string(),
            descricao: descricao.to_string(),
            ncm: ncm.to_string(),
            unidade: unidade.to_string(),
            preco_custo_centavos: preco_custo.centavos(),
            preco_venda_centavos: preco_venda.centavos(),
            categoria: categoria.clone(),
            marca: marca.clone(),
            controla_estoque,
            c_class_trib: classe_trib.as_ref().map(|c| c.as_str().to_string()),
            occurred_at: Utc::now(),
        });

        Ok(Self {
            id,
            version: 0,
            events,
            sku,
            descricao,
            ncm,
            unidade,
            preco_custo,
            preco_venda,
            categoria,
            marca,
            ativo: true,
            controla_estoque,
            classe_trib,
        })
    }

    pub fn atualizar_precos(
        &mut self,
        preco_custo: Dinheiro,
        preco_venda: Dinheiro,
    ) -> DomainResult<()> {
        if !self.ativo {
            return Err(DomainError::BusinessRule(
                "Produto inativo não pode ter preços atualizados".into(),
            ));
        }

        if preco_venda.centavos() <= 0 {
            return Err(DomainError::Validation(
                "Preço de venda deve ser positivo".into(),
            ));
        }

        if preco_custo.centavos() < 0 {
            return Err(DomainError::Validation(
                "Preço de custo não pode ser negativo".into(),
            ));
        }

        self.preco_custo = preco_custo;
        self.preco_venda = preco_venda;

        self.events.raise(CatalogoEvent::PrecosAtualizados {
            produto_id: self.id.to_string(),
            preco_custo_centavos: preco_custo.centavos(),
            preco_venda_centavos: preco_venda.centavos(),
            occurred_at: Utc::now(),
        });

        Ok(())
    }

    #[allow(clippy::too_many_arguments)] // flat args mirror the command payload
    pub fn atualizar(
        &mut self,
        sku: String,
        descricao: String,
        ncm: String,
        unidade: String,
        categoria: String,
        marca: Option<String>,
        controla_estoque: bool,
        classe_trib: Option<String>,
    ) -> DomainResult<()> {
        if !self.ativo {
            return Err(DomainError::BusinessRule(
                "Produto inativo não pode ser atualizado".into(),
            ));
        }

        self.sku = Sku::try_from(sku)?;
        self.descricao = NomeNaoVazio::try_from(descricao)?;
        self.ncm = Ncm::try_from(ncm)?;
        self.unidade = Unidade::try_from(unidade)?;
        self.categoria = categoria.clone();
        self.marca = marca.clone();
        self.controla_estoque = controla_estoque;
        self.classe_trib = classe_trib.map(ClasseTributaria::try_from).transpose()?;

        self.events.raise(CatalogoEvent::ProdutoAtualizado {
            produto_id: self.id.to_string(),
            sku: self.sku.to_string(),
            descricao: self.descricao.to_string(),
            ncm: self.ncm.to_string(),
            unidade: self.unidade.to_string(),
            categoria,
            marca,
            controla_estoque,
            c_class_trib: self.classe_trib.as_ref().map(|c| c.as_str().to_string()),
            occurred_at: Utc::now(),
        });

        Ok(())
    }

    pub fn desativar(&mut self) -> DomainResult<()> {
        if !self.ativo {
            return Err(DomainError::BusinessRule("Produto já está inativo".into()));
        }

        self.ativo = false;

        self.events.raise(CatalogoEvent::ProdutoDesativado {
            produto_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });

        Ok(())
    }

    pub fn reativar(&mut self) -> DomainResult<()> {
        if self.ativo {
            return Err(DomainError::BusinessRule("Produto já está ativo".into()));
        }

        self.ativo = true;

        self.events.raise(CatalogoEvent::ProdutoReativado {
            produto_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pharos_core::AggregateRoot;

    use super::*;

    fn produto_valido() -> Produto {
        Produto::cadastrar(
            "SKU-001".into(),
            "Pastilha de freio".into(),
            "87083090".into(),
            "UN".into(),
            Dinheiro::from_centavos(5000),
            Dinheiro::from_centavos(9000),
            "Freios".into(),
            Some("Fras-le".into()),
            true,
            None,
        )
        .expect("produto válido")
    }

    #[test]
    fn cadastrar_gera_evento_e_produto_ativo() {
        let p = produto_valido();
        assert!(p.ativo());
        assert_eq!(p.pending_events().len(), 1);
        assert!(matches!(
            p.pending_events()[0],
            CatalogoEvent::ProdutoCadastrado { .. }
        ));
    }

    #[test]
    fn cadastrar_preco_venda_nao_positivo_retorna_erro() {
        let r = Produto::cadastrar(
            "SKU-001".into(),
            "Pastilha".into(),
            "87083090".into(),
            "UN".into(),
            Dinheiro::from_centavos(5000),
            Dinheiro::from_centavos(0),
            "Freios".into(),
            None,
            true,
            None,
        );
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn cadastrar_preco_custo_negativo_retorna_erro() {
        let r = Produto::cadastrar(
            "SKU-001".into(),
            "Pastilha".into(),
            "87083090".into(),
            "UN".into(),
            Dinheiro::from_centavos(-1),
            Dinheiro::from_centavos(9000),
            "Freios".into(),
            None,
            true,
            None,
        );
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn cadastrar_ncm_invalido_retorna_erro() {
        let r = Produto::cadastrar(
            "SKU-001".into(),
            "Pastilha".into(),
            "123".into(),
            "UN".into(),
            Dinheiro::from_centavos(5000),
            Dinheiro::from_centavos(9000),
            "Freios".into(),
            None,
            true,
            None,
        );
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn cadastrar_classe_tributaria_invalida_retorna_erro() {
        let r = Produto::cadastrar(
            "SKU-001".into(),
            "Produto".into(),
            "87083090".into(),
            "UN".into(),
            Dinheiro::from_centavos(5000),
            Dinheiro::from_centavos(9000),
            "Geral".into(),
            None,
            true,
            Some("12A".into()), // cClassTrib exige 6 dígitos
        );
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn atualizar_classe_tributaria_valida_e_removivel() {
        let mut p = produto_valido();
        p.atualizar(
            "SKU-1".into(),
            "Produto".into(),
            "87083090".into(),
            "UN".into(),
            "Geral".into(),
            None,
            true,
            Some("200003".into()),
        )
        .expect("classe válida");
        assert_eq!(p.classe_trib().map(|c| c.as_str()), Some("200003"));

        // Remover a classe (None) volta ao padrão integral.
        p.atualizar(
            "SKU-1".into(),
            "Produto".into(),
            "87083090".into(),
            "UN".into(),
            "Geral".into(),
            None,
            true,
            None,
        )
        .expect("sem classe");
        assert!(p.classe_trib().is_none());
    }

    #[test]
    fn atualizar_precos_altera_valores_e_gera_evento() {
        let mut p = produto_valido();
        p.atualizar_precos(
            Dinheiro::from_centavos(6000),
            Dinheiro::from_centavos(11000),
        )
        .expect("atualizar preços");
        assert_eq!(p.preco_custo().centavos(), 6000);
        assert_eq!(p.preco_venda().centavos(), 11000);
        assert!(matches!(
            p.pending_events().last(),
            Some(CatalogoEvent::PrecosAtualizados { .. })
        ));
    }

    #[test]
    fn atualizar_precos_produto_inativo_retorna_erro() {
        let mut p = produto_valido();
        p.desativar().expect("desativar");
        assert!(matches!(
            p.atualizar_precos(Dinheiro::from_centavos(1), Dinheiro::from_centavos(2)),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn atualizar_dados_produto_inativo_retorna_erro() {
        let mut p = produto_valido();
        p.desativar().expect("desativar");
        assert!(matches!(
            p.atualizar(
                "SKU-2".into(),
                "Nova descrição".into(),
                "87083090".into(),
                "UN".into(),
                "Freios".into(),
                None,
                true,
                None,
            ),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn atualizar_dados_normaliza_unidade() {
        let mut p = produto_valido();
        p.atualizar(
            "SKU-2".into(),
            "Disco de freio".into(),
            "8708-30.90".into(),
            "un".into(),
            "Freios".into(),
            Some("Bosch".into()),
            true,
            None,
        )
        .expect("atualizar");
        assert_eq!(p.unidade().as_str(), "UN");
        assert_eq!(p.ncm().as_str(), "87083090");
        assert_eq!(p.sku().as_str(), "SKU-2");
    }

    #[test]
    fn desativar_e_reativar_alternam_estado() {
        let mut p = produto_valido();
        p.desativar().expect("desativar");
        assert!(!p.ativo());
        p.reativar().expect("reativar");
        assert!(p.ativo());
    }

    #[test]
    fn desativar_produto_ja_inativo_retorna_erro() {
        let mut p = produto_valido();
        p.desativar().expect("desativar");
        assert!(matches!(p.desativar(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn reativar_produto_ativo_retorna_erro() {
        let mut p = produto_valido();
        assert!(matches!(p.reativar(), Err(DomainError::BusinessRule(_))));
    }
}
