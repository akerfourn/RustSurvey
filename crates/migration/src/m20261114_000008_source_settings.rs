use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut document = ColumnDef::new("document");
        let mut report = ColumnDef::new("report");
        if manager.get_database_backend() == sea_orm_migration::sea_orm::DbBackend::MySql {
            document.custom("LONGTEXT");
            report.custom("LONGTEXT");
        } else {
            document.text();
            report.text();
        }
        manager
            .create_table(
                Table::create()
                    .table("source_settings_contexts")
                    .col(ColumnDef::new("id").string_len(36).not_null().primary_key())
                    .col(ColumnDef::new("study_id").string_len(36).not_null())
                    .col(
                        ColumnDef::new("definition_revision")
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new("source_sha256").string_len(64).not_null())
                    .col(ColumnDef::new("sha256").string_len(64).not_null())
                    .col(ColumnDef::new("size").big_integer().not_null())
                    .col(ColumnDef::new("profile").string_len(64).not_null())
                    .col(ColumnDef::new("is_complete").boolean().not_null())
                    .col(document.not_null())
                    .col(report.not_null())
                    .col(ColumnDef::new("created_at").big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_settings_context_revision")
                            .from(
                                "source_settings_contexts",
                                ("study_id", "definition_revision"),
                            )
                            .to("definition_revisions", ("study_id", "revision")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_settings_context_content")
                    .table("source_settings_contexts")
                    .col("study_id")
                    .col("definition_revision")
                    .col("profile")
                    .col("sha256")
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("source_settings_contexts").to_owned())
            .await
    }
}
