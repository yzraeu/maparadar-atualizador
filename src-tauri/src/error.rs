use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Falha na comunicação com o servidor: {0}")]
    Network(String),
    #[error("Usuário ou senha inválidos.")]
    InvalidLogin,
    #[error("Sessão expirada. Faça login novamente.")]
    Unauthorized,
    #[error("Nenhum ponto encontrado para os tipos selecionados.")]
    EmptyExport,
    #[error("Nenhum dispositivo GPS compatível encontrado.")]
    DeviceNotFound,
    #[error("Erro no servidor: {0}")]
    Api(String),
    #[error("Erro ao acessar arquivos: {0}")]
    Io(String),
    #[error("Erro na sessão local: {0}")]
    Session(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}
