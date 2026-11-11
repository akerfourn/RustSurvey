use super::AppError;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait Runtime: Send + Sync {
    fn now(&self) -> Result<i64, AppError>;
    fn random(&self) -> Result<[u8; 32], AppError>;
}
pub struct SystemRuntime;
impl Runtime for SystemRuntime {
    fn now(&self) -> Result<i64, AppError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AppError::internal())?
            .as_secs()
            .try_into()
            .map_err(|_| AppError::internal())
    }
    fn random(&self) -> Result<[u8; 32], AppError> {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(|_| AppError::internal())?;
        Ok(bytes)
    }
}
pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub(super) fn identifier(runtime: &dyn Runtime) -> Result<String, AppError> {
    let random = runtime.random()?;
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&random[..16]);
    Ok(uuid::Builder::from_random_bytes(bytes)
        .into_uuid()
        .to_string())
}
