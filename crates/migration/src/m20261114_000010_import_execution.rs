use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut document = ColumnDef::new("execution_document");
        if manager.get_database_backend() == sea_orm_migration::sea_orm::DbBackend::MySql {
            document.custom("LONGTEXT");
        } else {
            document.text();
        }
        manager
            .alter_table(Table::alter().table("runs").add_column(document).to_owned())
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table("runs")
                    .drop_column("execution_document")
                    .to_owned(),
            )
            .await
    }
}
