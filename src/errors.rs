/// Runtime error handling for the application
#[derive(Debug)]
#[allow(dead_code)]
pub enum RuntimeError {
    Io(std::io::Error),
    Lua(mlua::Error),
}

/// Result type for the application
pub type AppResult<T> = std::result::Result<T, RuntimeError>;

/// Convert RuntimeError to a string
impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RuntimeError::Io(err) => std::fmt::Display::fmt(err, f),
            RuntimeError::Lua(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

/// Convert std::io::Error to RuntimeError
impl From<std::io::Error> for RuntimeError {
    fn from(kind: std::io::Error) -> Self {
        RuntimeError::Io(kind)
    }
}

/// Convert mlua::Error to RuntimeError
impl From<mlua::Error> for RuntimeError {
    fn from(kind: mlua::Error) -> Self {
        RuntimeError::Lua(kind)
    }
}

/// Create a new `Not Found` error
pub fn not_found(error: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, error)
}

/// Extension trait to handle `Not Found` errors
pub trait NotFoundExt<T> {
    fn ok_or_not_found(self, error: &str) -> Result<T, std::io::Error>;
}

/// Implement NotFoundExt for Option
impl<T> NotFoundExt<T> for Option<T> {
    fn ok_or_not_found(self, error: &str) -> Result<T, std::io::Error> {
        self.ok_or_else(|| not_found(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_error_display_io() {
        let error = RuntimeError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "io error",
        ));
        assert_eq!(format!("{}", error), "io error");
    }

    #[test]
    fn test_runtime_error_display_lua() {
        let error = RuntimeError::Lua(mlua::Error::RuntimeError("lua error".to_string()));
        assert_eq!(format!("{}", error), "runtime error: lua error");
    }

    #[test]
    fn test_runtime_error_from_io() {
        let error = not_found("io error");
        let error: RuntimeError = error.into();
        assert!(matches!(error, RuntimeError::Io(_)));
    }

    #[test]
    fn test_runtime_error_from_lua() {
        let error = mlua::Error::RuntimeError("lua error".to_string());
        let error: RuntimeError = error.into();
        assert!(matches!(error, RuntimeError::Lua(_)));
    }

    #[test]
    fn test_option_not_found_ext_some() {
        let value = Some(42);
        let result = value.ok_or_not_found("not found");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_option_not_found_ext_none() {
        let value: Option<i32> = None;
        let result = value.ok_or_not_found("test error");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }
}
