//! Versioned, portable database migrations shared by the server and its tests.

pub use sea_orm_migration::prelude::*;

mod m20261111_000001_authoring;
mod m20261111_000002_import_sources;
mod m20261113_000003_media;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261111_000001_authoring::Migration),
            Box::new(m20261111_000002_import_sources::Migration),
            Box::new(m20261113_000003_media::Migration),
        ]
    }
}
