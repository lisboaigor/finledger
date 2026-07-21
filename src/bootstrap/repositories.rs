use std::sync::Arc;

use pharos_postgres::Pool;

use crate::backoffice::repository::BackofficeRepository;
use crate::bi::infrastructure::repository::PostgresBiRepository;
use crate::catalogo::application::handler::{PrecificacaoRepository, ProdutoRepository};
use crate::catalogo::infrastructure::precificacao_repository::PostgresPrecificacaoRepository;
use crate::catalogo::infrastructure::repository::PostgresProdutoRepository;
use crate::compras::infrastructure::repository::PostgresPedidoCompraRepository;
use crate::crm::infrastructure::repository::PostgresClienteRepository;
use crate::estoque::infrastructure::repository::PostgresEstoqueRepository;
use crate::financeiro::infrastructure::repository::{
    PostgresContaPagarRepository, PostgresContaReceberRepository,
};
use crate::fiscal::infrastructure::repository::PostgresNotaFiscalRepository;
use crate::fornecedores::infrastructure::repository::PostgresFornecedorRepository;
use crate::identity::infrastructure::repository::PostgresIdentityRepository;
use crate::orcamentos::infrastructure::repository::PostgresOrcamentoRepository;
use crate::tenants::repository::TenantRepository;
use crate::vendas::infrastructure::repository::PostgresVendaRepository;

pub type BackofficeRepositoryArc = Arc<BackofficeRepository>;
pub type ClienteRepository = Arc<PostgresClienteRepository>;
pub type EstoqueRepository = Arc<PostgresEstoqueRepository>;
pub type VendaRepository = Arc<PostgresVendaRepository>;
pub type FornecedorRepository = Arc<PostgresFornecedorRepository>;
pub type OrcamentoRepository = Arc<PostgresOrcamentoRepository>;
pub type PedidoCompraRepository = Arc<PostgresPedidoCompraRepository>;
pub type ContaReceberRepository = Arc<PostgresContaReceberRepository>;
pub type ContaPagarRepository = Arc<PostgresContaPagarRepository>;
pub type NotaFiscalRepository = Arc<PostgresNotaFiscalRepository>;
pub type UsuarioRepository = Arc<PostgresIdentityRepository>;
pub type TenantRepositoryArc = Arc<TenantRepository>;
pub type BiRepository = Arc<PostgresBiRepository>;

pub struct Repositories {
    pub backoffice: BackofficeRepositoryArc,
    pub usuarios: UsuarioRepository,
    pub produtos: ProdutoRepository,
    pub precificacao: PrecificacaoRepository,
    pub clientes: ClienteRepository,
    pub estoque: EstoqueRepository,
    pub vendas: VendaRepository,
    pub fornecedores: FornecedorRepository,
    pub orcamentos: OrcamentoRepository,
    pub pedidos: PedidoCompraRepository,
    pub contas_receber: ContaReceberRepository,
    pub contas_pagar: ContaPagarRepository,
    pub notas_fiscais: NotaFiscalRepository,
    pub tenants: TenantRepositoryArc,
    pub bi: BiRepository,
}

impl Repositories {
    pub fn new(pool: &Pool) -> Self {
        Self {
            backoffice: Arc::new(BackofficeRepository::new(pool.clone())),
            usuarios: Arc::new(PostgresIdentityRepository::new(pool.clone())),
            produtos: Arc::new(PostgresProdutoRepository::new(pool.clone())),
            precificacao: Arc::new(PostgresPrecificacaoRepository::new(pool.clone())),
            clientes: Arc::new(PostgresClienteRepository::new(pool.clone())),
            estoque: Arc::new(PostgresEstoqueRepository::new(pool.clone())),
            vendas: Arc::new(PostgresVendaRepository::new(pool.clone())),
            fornecedores: Arc::new(PostgresFornecedorRepository::new(pool.clone())),
            orcamentos: Arc::new(PostgresOrcamentoRepository::new(pool.clone())),
            pedidos: Arc::new(PostgresPedidoCompraRepository::new(pool.clone())),
            contas_receber: Arc::new(PostgresContaReceberRepository::new(pool.clone())),
            contas_pagar: Arc::new(PostgresContaPagarRepository::new(pool.clone())),
            notas_fiscais: Arc::new(PostgresNotaFiscalRepository::new(pool.clone())),
            tenants: Arc::new(TenantRepository::new(pool.clone())),
            bi: Arc::new(PostgresBiRepository::new(pool.clone())),
        }
    }
}
