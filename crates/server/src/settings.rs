//! Server-only configuration: process environment takes precedence over `.env`.
use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read},
    path::Path,
};

pub struct Settings {
    values: BTreeMap<OsString, OsString>,
}

impl Settings {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        match File::open(path) {
            Ok(file) => Self::from_reader(file, env::vars_os()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Self::from_reader(io::empty(), env::vars_os())
            }
            Err(error) => Err(io::Error::new(
                error.kind(),
                "cannot open .env configuration",
            )),
        }
    }

    /// Compose configuration with an explicit environment, without changing process state.
    pub fn from_reader(
        reader: impl Read,
        environment: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> io::Result<Self> {
        let mut values = BTreeMap::new();
        for entry in dotenvy::from_read_iter(reader) {
            let (key, value) = entry.map_err(|error| match error {
                dotenvy::Error::Io(error) => {
                    io::Error::new(error.kind(), "cannot read .env configuration")
                }
                _ => io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid .env syntax; check key/value pairs and quoting",
                ),
            })?;
            values
                .entry(OsString::from(key))
                .or_insert(OsString::from(value));
        }
        values.extend(environment);
        Ok(Self { values })
    }

    pub fn get(&self, name: &str) -> io::Result<Option<String>> {
        let Some(value) = self.values.get(OsStr::new(name)) else {
            return Ok(None);
        };
        let value = value.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, format!("{name} must be UTF-8"))
        })?;
        Ok(Some(value.to_owned()))
    }

    pub fn value_or(&self, name: &str, default: &str) -> io::Result<String> {
        Ok(self.get(name)?.unwrap_or_else(|| default.to_owned()))
    }

    pub fn boolean(&self, name: &str, default: bool) -> io::Result<bool> {
        match self.get(name)?.as_deref() {
            None => Ok(default),
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            Some(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must be true or false"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;
    use std::{ffi::OsString, io};

    #[test]
    fn test_dotenv_should_read_quoted_credentials_and_comments_without_changing_the_environment() {
        let original = std::env::var_os("ADMIN_USERNAME");
        let source =
            "# Local credentials\nADMIN_USERNAME=admin\nADMIN_PASSWORD='dev password # literal'\n";
        let settings = Settings::from_reader(source.as_bytes(), []).unwrap();
        assert_eq!(
            settings.get("ADMIN_USERNAME").unwrap().as_deref(),
            Some("admin")
        );
        assert_eq!(
            settings.get("ADMIN_PASSWORD").unwrap().as_deref(),
            Some("dev password # literal")
        );
        assert_eq!(
            settings.value_or("BIND_ADDRESS", "127.0.0.1:3000").unwrap(),
            "127.0.0.1:3000"
        );
        assert!(
            std::env::var_os("ADMIN_USERNAME") == original,
            "dotenv must not mutate the process environment"
        );
    }

    #[test]
    fn test_environment_should_override_dotenv_values_including_empty_values() {
        let source = "ADMIN_USERNAME=file\nADMIN_PASSWORD=file-password\n";
        let settings = Settings::from_reader(
            source.as_bytes(),
            [
                (
                    OsString::from("ADMIN_USERNAME"),
                    OsString::from("environment"),
                ),
                (OsString::from("ADMIN_PASSWORD"), OsString::from("")),
            ],
        )
        .unwrap();
        assert_eq!(
            settings.get("ADMIN_USERNAME").unwrap().as_deref(),
            Some("environment")
        );
        assert_eq!(settings.get("ADMIN_PASSWORD").unwrap().as_deref(), Some(""));
        assert_eq!(settings.get("BIND_ADDRESS").unwrap(), None);
    }

    #[test]
    fn test_invalid_dotenv_should_fail_without_disclosing_the_credential() {
        let source = "ADMIN_PASSWORD='synthetic-secret-with-unclosed-quote\n";
        let error = Settings::from_reader(source.as_bytes(), []).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            error.to_string(),
            "invalid .env syntax; check key/value pairs and quoting"
        );
    }

    #[test]
    fn test_example_should_configure_a_local_account_without_fixing_a_development_port() {
        let source = include_str!("../../../.env.example");
        let settings = Settings::from_reader(source.as_bytes(), []).unwrap();
        assert_eq!(
            settings.get("ADMIN_USERNAME").unwrap().as_deref(),
            Some("admin")
        );
        assert_eq!(
            settings.get("ADMIN_PASSWORD").unwrap().as_deref(),
            Some("rustsurvey-dev-only")
        );
        assert_eq!(settings.get("BIND_ADDRESS").unwrap(), None);
        assert!(!settings.boolean("DEV_ALLOW_WEAK_PASSWORDS", false).unwrap());
        assert_eq!(
            settings.get("DATABASE_URL").unwrap().as_deref(),
            Some(crate::storage::DEFAULT_DATABASE_URL)
        );
        assert_eq!(
            settings.get("OBJECT_STORE_BACKEND").unwrap().as_deref(),
            Some("file")
        );
        assert_eq!(
            settings.get("OBJECT_STORE_DIRECTORY").unwrap().as_deref(),
            Some(".data/media")
        );
    }

    #[test]
    fn test_boolean_setting_should_require_explicit_true_or_false() {
        let source = "ENABLED=true\nDISABLED=false\nINVALID=synthetic-private-value\n";
        let settings = Settings::from_reader(source.as_bytes(), []).unwrap();
        assert!(settings.boolean("ENABLED", false).unwrap());
        assert!(!settings.boolean("DISABLED", true).unwrap());
        assert!(!settings.boolean("MISSING", false).unwrap());
        let error = settings.boolean("INVALID", false).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), "INVALID must be true or false");
    }
}
