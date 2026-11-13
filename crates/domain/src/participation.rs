//! Campaign access and repeat participation, without HTTP or storage concerns.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub enum ParticipationPolicy {
    #[default]
    Open,
    BrowserOnce,
    #[serde(rename_all = "camelCase")]
    Invitation {
        persistence: bool,
        allow_edit_after_submit: bool,
    },
}

impl ParticipationPolicy {
    pub fn validate(&self) -> Result<(), &'static str> {
        if matches!(
            self,
            Self::Invitation {
                persistence: false,
                allow_edit_after_submit: true,
            }
        ) {
            return Err("Modifier une réponse exige de la relier à son invitation.");
        }
        Ok(())
    }

    pub fn requires_invitation(&self) -> bool {
        matches!(self, Self::Invitation { .. })
    }

    pub fn persists_response(&self) -> bool {
        matches!(
            self,
            Self::Invitation {
                persistence: true,
                ..
            }
        )
    }

    pub fn allows_edit(&self) -> bool {
        matches!(
            self,
            Self::Invitation {
                persistence: true,
                allow_edit_after_submit: true
            }
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvitationLimits {
    pub max_uses: Option<u32>,
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
}

impl InvitationLimits {
    pub fn validate(&self, policy: &ParticipationPolicy) -> Result<(), &'static str> {
        if !policy.requires_invitation() {
            return Err("Cette campagne n'utilise pas d'invitations.");
        }
        if self
            .max_uses
            .is_some_and(|maximum| maximum == 0 || maximum > 100_000)
        {
            return Err("Le nombre de soumissions doit être compris entre 1 et 100 000.");
        }
        if policy.persists_response() && self.max_uses != Some(1) {
            return Err("Une invitation avec reprise correspond à une seule réponse.");
        }
        if self.valid_from.is_some_and(|date| date < 0)
            || self.valid_until.is_some_and(|date| date < 0)
        {
            return Err("Les dates de validité doivent être positives.");
        }
        if let (Some(start), Some(end)) = (self.valid_from, self.valid_until)
            && start >= end
        {
            return Err("La fin de validité doit être postérieure au début.");
        }
        Ok(())
    }

    /// Validity is half-open: valid_from <= now < valid_until.
    pub fn is_active(&self, now: i64, revoked: bool) -> bool {
        !revoked
            && self.valid_from.is_none_or(|start| now >= start)
            && self.valid_until.is_none_or(|end| now < end)
    }

    pub fn has_uses_left(&self, uses: i64) -> bool {
        uses >= 0
            && self
                .max_uses
                .is_none_or(|maximum| uses < i64::from(maximum))
    }
}
