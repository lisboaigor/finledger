//! Motor tributário multi-regime da reforma tributária (EC 132/2023, LC 214/2025).
//!
//! Divisão de responsabilidades:
//! - resolução de alíquotas é infraestrutura (`fiscal::infrastructure::aliquotas`)
//!   — lookup no Postgres por vigência/UF/regime/classe;
//! - o cálculo em si é puro (`MotorTributario`) sobre um snapshot já resolvido,
//!   testável sem banco, espelhando o padrão `SefazClient`.

pub mod aliquota;
pub mod classe_tributaria;
pub mod fase_transicao;
pub mod motor;
pub mod perfil_fiscal;

pub use aliquota::{Aliquota, TributoTipo};
pub use classe_tributaria::{ClasseTributaria, ClasseTributariaInfo};
pub use fase_transicao::FaseTransicao;
pub use motor::{AliquotasItem, ContextoFiscal, MotorTributario};
pub use perfil_fiscal::{CodigoMunicipio, Crt, PerfilFiscal, RegimeTributario, Uf};
