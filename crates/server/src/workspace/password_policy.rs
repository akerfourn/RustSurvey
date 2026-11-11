use std::io;

#[derive(Clone, Copy, Debug, Default)]
pub struct PasswordPolicy {
    allow_weak_passwords: bool,
}

impl PasswordPolicy {
    pub fn from_development_flag(enabled: bool) -> io::Result<Self> {
        Self::for_build(enabled, cfg!(debug_assertions))
    }

    fn for_build(enabled: bool, debug_build: bool) -> io::Result<Self> {
        if enabled && !debug_build {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "DEV_ALLOW_WEAK_PASSWORDS is only available in debug builds",
            ));
        }
        Ok(Self {
            allow_weak_passwords: enabled,
        })
    }

    pub(super) fn minimum_characters(self) -> usize {
        if self.allow_weak_passwords { 1 } else { 12 }
    }
}

#[cfg(test)]
mod tests {
    use super::PasswordPolicy;
    use std::io;

    #[test]
    fn test_default_policy_should_require_twelve_characters() {
        assert_eq!(PasswordPolicy::default().minimum_characters(), 12);
        for debug_build in [false, true] {
            let policy = PasswordPolicy::for_build(false, debug_build).unwrap();
            assert_eq!(policy.minimum_characters(), 12);
        }
    }

    #[test]
    fn test_development_policy_should_accept_short_but_nonempty_passwords() {
        let policy = PasswordPolicy::for_build(true, true).unwrap();
        assert_eq!(policy.minimum_characters(), 1);
    }

    #[test]
    fn test_release_build_should_reject_the_weak_password_flag() {
        let error = PasswordPolicy::for_build(true, false).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            error.to_string(),
            "DEV_ALLOW_WEAK_PASSWORDS is only available in debug builds"
        );
    }
}
