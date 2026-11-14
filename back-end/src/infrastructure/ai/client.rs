/*
CLIENTE HTTP PARA O OLLAMA.

Configuração (URL, modelo, timeout) chega pronta de `config::settings`
— nenhuma URL ou modelo fixo espalhado pelo código. Falhas de conexão,
timeout e respostas inválidas são tratadas explicitamente como
`ErroIa::Indisponivel`/`RespostaInvalida`: a IA é um componente OPCIONAL
e indisponibilidade não deve interromper o monitoramento.

Não há RAG, embeddings ou busca semântica: o cliente envia UM prompt
e recebe UM texto.
*/

use std::time::Duration;

use thiserror::Error;

use super::models::{OllamaGeneratePedido, OllamaGenerateResposta, OllamaOpcoes};

#[derive(Debug, Error)]
pub enum ErroIa {
    #[error("serviço de IA indisponível: {0}")]
    Indisponivel(String),

    #[error("tempo limite excedido ao consultar a IA")]
    Timeout,

    #[error("resposta inválida do serviço de IA: {0}")]
    RespostaInvalida(String),
}

#[derive(Debug, Clone)]
pub struct OllamaClient {
    base_url: String,
    modelo: String,
    timeout: Duration,
    http: reqwest::Client,
}

impl OllamaClient {
    /// Constrói o cliente a partir das configurações do projeto.
    /// Falha (de configuração) quando a URL não pode ser parseada ou o
    /// timeout não é representável.
    pub fn novo(base_url: &str, modelo: &str, timeout_secs: u64) -> Result<Self, String> {
        let base_url = base_url.trim().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err("URL do Ollama não pode ser vazia".to_string());
        }
        if modelo.trim().is_empty() {
            return Err("modelo do Ollama não pode ser vazio".to_string());
        }
        if timeout_secs == 0 {
            return Err("timeout da IA deve ser maior que zero".to_string());
        }

        let timeout = Duration::from_secs(timeout_secs);
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("falha ao criar cliente HTTP da IA: {e}"))?;

        Ok(Self {
            base_url,
            modelo: modelo.trim().to_string(),
            timeout,
            http,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn modelo(&self) -> &str {
        &self.modelo
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Verificação leve de disponibilidade: a raiz do serviço do Ollama
    /// responde 200 com "Ollama is running". Qualquer falha → `false`
    /// (nunca pânico, nunca bloqueio longo além do timeout do cliente).
    pub async fn disponivel(&self) -> bool {
        match self.http.get(&self.base_url).send().await {
            Ok(resposta) => resposta.status().is_success(),
            Err(_) => false,
        }
    }

    /// Envia um prompt e devolve o texto gerado pelo modelo.
    pub async fn gerar(&self, prompt: &str) -> Result<String, ErroIa> {
        let pedido = OllamaGeneratePedido {
            model: self.modelo.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: OllamaOpcoes { temperature: 0.2 },
        };

        let url = format!("{}/api/generate", self.base_url);

        let resposta = self
            .http
            .post(&url)
            .json(&pedido)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ErroIa::Timeout
                } else {
                    ErroIa::Indisponivel(e.to_string())
                }
            })?;

        if !resposta.status().is_success() {
            return Err(ErroIa::Indisponivel(format!(
                "o serviço respondeu com status {}",
                resposta.status()
            )));
        }

        let corpo: OllamaGenerateResposta = resposta
            .json()
            .await
            .map_err(|e| ErroIa::RespostaInvalida(format!("JSON não interpretável: {e}")))?;

        let texto = corpo.response.trim().to_string();
        if texto.is_empty() {
            return Err(ErroIa::RespostaInvalida(
                "o modelo devolveu uma resposta vazia".to_string(),
            ));
        }

        Ok(texto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cliente_rejeita_configuracao_invalida() {
        assert!(OllamaClient::novo("", "llama3.2", 10).is_err());
        assert!(OllamaClient::novo("  ", "llama3.2", 10).is_err());
        assert!(OllamaClient::novo("http://localhost:11434", " ", 10).is_err());
        assert!(OllamaClient::novo("http://localhost:11434", "m", 0).is_err());
        assert!(OllamaClient::novo("http://localhost:11434/", "m", 30).is_ok());
    }

    #[test]
    fn url_final_e_normalizada() {
        let cliente = OllamaClient::novo("http://localhost:11434/", "llama3.2", 5).unwrap();
        assert_eq!(cliente.base_url(), "http://localhost:11434");
        assert_eq!(cliente.modelo(), "llama3.2");
        assert_eq!(cliente.timeout(), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn servico_inacessivel_e_tratado_como_indisponivel() {
        // Porta reservada (1) — conexão falha rapidamente, sem Ollama real.
        let cliente = OllamaClient::novo("http://127.0.0.1:1", "modelo-teste", 2).unwrap();
        assert!(!cliente.disponivel().await);
        let erro = cliente.gerar("prompt").await.unwrap_err();
        assert!(matches!(erro, ErroIa::Indisponivel(_) | ErroIa::Timeout));
    }
}
