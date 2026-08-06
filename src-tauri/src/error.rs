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

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Network(_) => "network",
            AppError::InvalidLogin => "invalidLogin",
            AppError::Unauthorized => "unauthorized",
            AppError::EmptyExport => "emptyExport",
            AppError::DeviceNotFound => "deviceNotFound",
            AppError::Api(_) => "api",
            AppError::Io(_) => "io",
            AppError::Session(_) => "session",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            AppError::Api(format!("resposta inválida do servidor: {e}"))
        } else {
            AppError::Network(e.to_string())
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_kind_and_message() {
        let json = serde_json::to_string(&AppError::InvalidLogin).unwrap();
        assert_eq!(json, r#"{"kind":"invalidLogin","message":"Usuário ou senha inválidos."}"#);
    }

    #[test]
    fn serializes_unauthorized() {
        let json = serde_json::to_string(&AppError::Unauthorized).unwrap();
        assert_eq!(json, r#"{"kind":"unauthorized","message":"Sessão expirada. Faça login novamente."}"#);
    }
}
