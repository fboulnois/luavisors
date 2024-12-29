#[derive(Debug)]
#[allow(dead_code)]
pub enum RuntimeError {
    Io(std::io::Error),
    Lua(mlua::Error),
}

pub type AppResult<T> = std::result::Result<T, RuntimeError>;

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RuntimeError::Io(err) => std::fmt::Display::fmt(err, f),
            RuntimeError::Lua(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl From<std::io::Error> for RuntimeError {
    fn from(kind: std::io::Error) -> Self {
        RuntimeError::Io(kind)
    }
}

impl From<mlua::Error> for RuntimeError {
    fn from(kind: mlua::Error) -> Self {
        RuntimeError::Lua(kind)
    }
}

pub fn not_found(error: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, error)
}

pub trait NotFoundExt<T> {
    fn ok_or_not_found(self, error: &str) -> Result<T, std::io::Error>;
}

impl<T> NotFoundExt<T> for Option<T> {
    fn ok_or_not_found(self, error: &str) -> Result<T, std::io::Error> {
        self.ok_or_else(|| not_found(error))
    }
}
