use crate::settings::Settings;
use rustsurvey_object_store::{
    ObjectKey, ObjectMetadata,
    s3::{S3Configuration, S3Credentials, S3ObjectStore},
};
use std::io;

pub(super) fn configuration(settings: &Settings) -> io::Result<S3Configuration> {
    S3Configuration::new(
        &required(settings, "S3_ENDPOINT")?,
        &required(settings, "S3_BUCKET")?,
        &settings.value_or("S3_PREFIX", "rustsurvey/v1")?,
        &required(settings, "S3_REGION")?,
        S3Credentials {
            access_key_id: required(settings, "S3_ACCESS_KEY_ID")?,
            secret_access_key: required(settings, "S3_SECRET_ACCESS_KEY")?,
            session_token: settings.get("S3_SESSION_TOKEN")?,
        },
        settings.boolean("S3_ALLOW_HTTP_LOOPBACK", false)?,
    )
    .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn required(settings: &Settings, name: &str) -> io::Result<String> {
    settings
        .get(name)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} is required in s3 mode"),
            )
        })
}

pub(super) async fn verify(store: &S3ObjectStore) -> io::Result<()> {
    let mut random = [0; 32];
    getrandom::fill(&mut random)
        .map_err(|_| io::Error::other("cannot create S3 capability probe"))?;
    let digest = ObjectMetadata::for_bytes(&random)
        .map_err(|_| io::Error::other("cannot create S3 capability probe"))?;
    let key = ObjectKey::parse(&digest.sha256)
        .map_err(|_| io::Error::other("cannot create S3 capability probe"))?;
    store.verify_immutable_writes(&key).await.map_err(|_| {
        io::Error::other("S3 capability check failed; verify the endpoint, credentials, bucket permissions and SQL index")
    })
}
