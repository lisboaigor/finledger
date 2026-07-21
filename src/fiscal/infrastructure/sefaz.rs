use std::future::Future;

pub struct SefazResponse {
    pub chave: String,
    pub protocolo: String,
}

pub enum SefazError {
    Rejeicao { codigo: String, motivo: String },
    Indisponivel(String),
}

/// Port para o webservice da SEFAZ. Implementações reais fazem SOAP sobre HTTPS
/// com certificado A1/A3. Para testes e MVP usa-se o `StubSefazClient`.
pub trait SefazClient: Send + Sync + 'static {
    fn transmitir(
        &self,
        xml: String,
    ) -> impl Future<Output = Result<SefazResponse, SefazError>> + Send;
}

/// Autoriza toda NF imediatamente sem contato real com a SEFAZ.
/// Usar em desenvolvimento e homologação até o certificado estar disponível.
pub struct StubSefazClient;

impl SefazClient for StubSefazClient {
    async fn transmitir(&self, _xml: String) -> Result<SefazResponse, SefazError> {
        let id = uuid::Uuid::new_v4();
        let n = id.as_u128();
        // Chave fictícia de 44 dígitos: cUF(35-SP) + AAMM(2501) + CNPJ(14) + mod(55) + série+nNF(12) + cNF+DV(9)
        let chave = format!(
            "35250100000000000001550010{:09}{:09}",
            n % 1_000_000_000u128,
            (n >> 32) % 1_000_000_000u128
        );
        let protocolo = format!("{:015}", n % 1_000_000_000_000_000u128);
        Ok(SefazResponse { chave, protocolo })
    }
}
