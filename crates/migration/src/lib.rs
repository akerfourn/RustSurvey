//! Versioned, portable database migrations shared by the server and its tests.

pub use sea_orm_migration::prelude::*;

mod m20261111_000001_authoring;
mod m20261111_000002_import_sources;
mod m20261113_000003_media;
mod m20261113_000004_collection;
mod m20261113_000005_participation;
mod m20261114_000006_response_exports;
mod m20261114_000007_response_imports;
mod m20261114_000008_source_settings;
mod m20261114_000009_navigation;
mod m20261114_000010_import_execution;
mod m20261114_000011_s3_objects;
mod m20261116_000012_publications;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261111_000001_authoring::Migration),
            Box::new(m20261111_000002_import_sources::Migration),
            Box::new(m20261113_000003_media::Migration),
            Box::new(m20261113_000004_collection::Migration),
            Box::new(m20261113_000005_participation::Migration),
            Box::new(m20261114_000006_response_exports::Migration),
            Box::new(m20261114_000007_response_imports::Migration),
            Box::new(m20261114_000008_source_settings::Migration),
            Box::new(m20261114_000009_navigation::Migration),
            Box::new(m20261114_000010_import_execution::Migration),
            Box::new(m20261114_000011_s3_objects::Migration),
            Box::new(m20261116_000012_publications::Migration),
        ]
    }
}
