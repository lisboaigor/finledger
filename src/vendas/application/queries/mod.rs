pub mod buscar_venda;
pub mod listar_vendas;
pub mod listar_vendas_arquivadas;

pub use buscar_venda::{BuscarVenda, VendaDetalhes, VendaItemResult};
pub use listar_vendas::{ListarVendas, VendaResult};
pub use listar_vendas_arquivadas::{ListarVendasArquivadas, VendaArquivadaResult};
