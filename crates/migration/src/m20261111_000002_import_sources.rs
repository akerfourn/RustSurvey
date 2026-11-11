use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut xml = ColumnDef::new("xml");
        if manager.get_database_backend() == sea_orm_migration::sea_orm::DbBackend::MySql {
            xml.custom("LONGTEXT").not_null();
        } else {
            xml.text().not_null();
        }
        let mut report = ColumnDef::new("report");
        if manager.get_database_backend() == sea_orm_migration::sea_orm::DbBackend::MySql {
            report.custom("LONGTEXT").not_null();
        } else {
            report.text().not_null();
        }
        manager
            .create_table(
                Table::create()
                    .table("import_sources")
                    .col(
                        ColumnDef::new("study_id")
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(&mut xml)
                    .col(ColumnDef::new("sha256").string_len(64).not_null())
                    .col(&mut report)
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_import_study")
                            .from("import_sources", "study_id")
                            .to("studies", "id")
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("import_sources").to_owned())
            .await
    }
}
