//! A bounded public build, loaded once; HTTP paths never become filesystem paths.
mod assets;
mod response;

use crate::settings::Settings;
use axum::{Router, body::Bytes};
use std::{collections::BTreeMap, io, path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct WebAssets {
    files: Arc<BTreeMap<String, Asset>>,
}

struct Asset {
    body: Bytes,
    content_type: &'static str,
    cache_control: &'static str,
    etag: String,
}

impl WebAssets {
    pub async fn load(directory: impl Into<PathBuf>) -> io::Result<Self> {
        let directory = directory.into();
        tokio::task::spawn_blocking(move || assets::load(&directory))
            .await
            .map_err(|_| assets::invalid_build())?
    }

    pub fn routes(self) -> Router {
        Router::new().fallback(response::serve).with_state(self)
    }
}

pub async fn configured_assets(settings: &Settings) -> io::Result<Option<WebAssets>> {
    let Some(directory) = settings.get("STATIC_ASSETS_DIRECTORY")? else {
        return Ok(None);
    };
    if directory.is_empty() {
        return Err(assets::invalid_build());
    }
    Ok(Some(WebAssets::load(directory).await?))
}
