mod handlers;
mod repository;

pub(super) use handlers::{create, list, revoke};
pub(super) use repository::{by_response, by_secret};

use crate::workspace::AppError;
use rustsurvey_domain::participation::InvitationLimits;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Invitation {
    pub id: String,
    pub run_id: String,
    #[serde(flatten)]
    pub limits: InvitationLimits,
    pub uses: i64,
    pub revoked: bool,
    pub created_at: i64,
    #[serde(skip)]
    pub secret_hash: String,
    #[serde(skip)]
    pub response_id: Option<String>,
}

impl Invitation {
    pub fn check_active(&self, now: i64) -> Result<(), AppError> {
        if !self.limits.is_active(now, self.revoked) {
            return Err(super::participation::denied());
        }
        Ok(())
    }

    pub fn check_remaining(&self) -> Result<(), AppError> {
        if !self.limits.has_uses_left(self.uses) {
            return Err(super::participation::exhausted());
        }
        Ok(())
    }
}
