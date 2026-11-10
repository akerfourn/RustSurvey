//! Versioned, portable database migrations shared by the server and its tests.

pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // Register domain migrations here once the survey model has been agreed.
        Vec::new()
    }
}
