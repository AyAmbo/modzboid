use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Game error: {0}")]
    Game(String),
}

// Manual Serialize — required because derive(Serialize) conflicts with From<T> conversions
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        let kind = match self {
            AppError::Io(_) => "Io",
            AppError::Parse(_) => "Parse",
            AppError::NotFound(_) => "NotFound",
            AppError::Database(_) => "Database",
            AppError::Validation(_) => "Validation",
            AppError::Game(_) => "Game",
        };
        s.serialize_field("kind", kind)?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Parse(e.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(e: zip::result::ZipError) -> Self {
        AppError::Io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_error_serializes_to_kind_message() {
        let error = AppError::NotFound("config file".to_string());
        let json = serde_json::to_string(&error).expect("serialization failed");

        let value: serde_json::Value = serde_json::from_str(&json).expect("JSON parse failed");
        assert_eq!(value["kind"], "NotFound", "expected kind = 'NotFound'");
        assert!(
            value["message"].as_str().unwrap().contains("Not found:"),
            "expected message to contain 'Not found:', got: {}",
            value["message"]
        );

        let io_error = AppError::Io("permission denied".to_string());
        let json = serde_json::to_string(&io_error).expect("serialization failed");
        let value: serde_json::Value = serde_json::from_str(&json).expect("JSON parse failed");
        assert_eq!(value["kind"], "Io");
        assert!(value["message"].as_str().unwrap().contains("IO error:"));

        let db_error = AppError::Database("connection refused".to_string());
        let json = serde_json::to_string(&db_error).expect("serialization failed");
        let value: serde_json::Value = serde_json::from_str(&json).expect("JSON parse failed");
        assert_eq!(value["kind"], "Database");
    }

    #[test]
    fn from_io_error_works() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err = AppError::from(io_err);
        assert!(
            matches!(app_err, AppError::Io(_)),
            "expected AppError::Io variant"
        );
        assert!(
            app_err.to_string().contains("IO error:"),
            "error message should start with 'IO error:'"
        );
    }

    #[test]
    fn from_serde_error_works() {
        let serde_err = serde_json::from_str::<serde_json::Value>("not valid json {{{")
            .expect_err("expected a parse error");
        let app_err = AppError::from(serde_err);
        assert!(
            matches!(app_err, AppError::Parse(_)),
            "expected AppError::Parse variant"
        );
        assert!(
            app_err.to_string().contains("Parse error:"),
            "error message should start with 'Parse error:'"
        );
    }
}
