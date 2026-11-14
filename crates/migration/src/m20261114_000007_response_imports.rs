use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn document(name: &'static str, backend: sea_orm_migration::sea_orm::DbBackend) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    if backend == sea_orm_migration::sea_orm::DbBackend::MySql {
        column.custom("LONGTEXT");
    } else {
        column.text();
    }
    column.not_null();
    column
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let mut schema = document("schema_document", backend);
        let mut report = document("report_document", backend);
        manager
            .create_table(
                Table::create()
                    .table("response_imports")
                    .col(ColumnDef::new("id").string_len(36).not_null().primary_key())
                    .col(ColumnDef::new("study_id").string_len(36).not_null())
                    .col(
                        ColumnDef::new("definition_revision")
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new("source_sha256").string_len(64).not_null())
                    .col(ColumnDef::new("sha256").string_len(64).not_null())
                    .col(ColumnDef::new("object_key").string_len(64).not_null())
                    .col(ColumnDef::new("size").big_integer().not_null())
                    .col(ColumnDef::new("state").string_len(16).not_null())
                    .col(ColumnDef::new("row_count").big_integer().not_null())
                    .col(
                        ColumnDef::new("loaded_rows")
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new("write_version")
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new("writer_token").string_len(36))
                    .col(&mut schema)
                    .col(&mut report)
                    .col(ColumnDef::new("created_at").big_integer().not_null())
                    .col(ColumnDef::new("updated_at").big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_response_import_revision")
                            .from("response_imports", ("study_id", "definition_revision"))
                            .to("definition_revisions", ("study_id", "revision")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_response_import_content")
                    .table("response_imports")
                    .col("study_id")
                    .col("definition_revision")
                    .col("sha256")
                    .unique()
                    .to_owned(),
            )
            .await?;
        let mut row = document("document", backend);
        manager
            .create_table(
                Table::create()
                    .table("response_import_records")
                    .col(ColumnDef::new("batch_id").string_len(36).not_null())
                    .col(ColumnDef::new("ordinal").big_integer().not_null())
                    .col(ColumnDef::new("source_id").string_len(20))
                    .col(ColumnDef::new("status").string_len(16).not_null())
                    .col(ColumnDef::new("sha256").string_len(64).not_null())
                    .col(&mut row)
                    .primary_key(Index::create().col("batch_id").col("ordinal"))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_response_import_record")
                            .from("response_import_records", "batch_id")
                            .to("response_imports", "id")
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_response_import_source_id")
                    .table("response_import_records")
                    .col("batch_id")
                    .col("source_id")
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("response_import_records").to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table("response_imports").to_owned())
            .await
    }
}
