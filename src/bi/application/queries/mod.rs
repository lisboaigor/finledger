pub mod listar_alertas_bi;
pub mod obter_comercial_bi;
pub mod obter_estoque_bi;
pub mod obter_financeiro_bi;
pub mod obter_resumo_bi;

pub use listar_alertas_bi::{AlertaResult, ListarAlertasBi};
pub use obter_comercial_bi::{
    ClienteRiscoResult, ComercialBi, FunilResult, ObterComercialBi, OrcamentoExpirandoResult, RfmSegmentoResult,
    VendedorResult,
};
pub use obter_estoque_bi::{
    CategoriaGiroResult, EstoqueBi, EstoqueMortoResult, MatrizAbcXyzResult, ObterEstoqueBi, PedidoParadoResult,
    RupturaResult,
};
pub use obter_financeiro_bi::{
    AgingResult, CicloFinanceiroResult, DevedorResult, FinanceiroBi, ObterFinanceiroBi, SemanaFluxoResult,
};
pub use obter_resumo_bi::{BiResumoCompleto, BiResumoResult, ObterResumoBi, ReceitaDiaResult};
