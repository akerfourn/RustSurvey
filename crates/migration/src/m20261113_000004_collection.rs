use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn document(manager: &SchemaManager, name: &'static str) -> ColumnDef {
    let mut column = ColumnDef::new(name);
    if manager.get_database_backend() == sea_orm_migration::sea_orm::DbBackend::MySql {
        column.custom("LONGTEXT").not_null();
    } else {
        column.text().not_null();
    }
    column
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("runs")
                    .col(ColumnDef::new("id").string_len(36).not_null().primary_key())
                    .col(ColumnDef::new("study_id").string_len(36).not_null())
                    .col(
                        ColumnDef::new("definition_revision")
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new("profile").string_len(64).not_null())
                    .col(ColumnDef::new("is_open").boolean().not_null())
                    .col(
                        ColumnDef::new("write_version")
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new("created_at").big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_run_definition")
                            .from("runs", ("study_id", "definition_revision"))
                            .to("definition_revisions", ("study_id", "revision")),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_run_study")
                    .table("runs")
                    .col("study_id")
                    .col("created_at")
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table("responses")
                    .col(ColumnDef::new("id").string_len(36).not_null().primary_key())
                    .col(ColumnDef::new("run_id").string_len(36).not_null())
                    .col(ColumnDef::new("access_hash").string_len(64).not_null())
                    .col(ColumnDef::new("start_hash").string_len(64).not_null())
                    .col(ColumnDef::new("version").big_integer().not_null())
                    .col(ColumnDef::new("status").string_len(16).not_null())
                    .col(document(manager, "document"))
                    .col(ColumnDef::new("created_at").big_integer().not_null())
                    .col(ColumnDef::new("updated_at").big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_response_run")
                            .from("responses", "run_id")
                            .to("runs", "id"),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_response_run")
                    .table("responses")
                    .col("run_id")
                    .col("created_at")
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table("operation_receipts")
                    .col(ColumnDef::new("response_id").string_len(36).not_null())
                    .col(ColumnDef::new("operation_id").string_len(36).not_null())
                    .col(ColumnDef::new("request_hash").string_len(64).not_null())
                    .col(document(manager, "result"))
                    .col(ColumnDef::new("expires_at").big_integer().not_null())
                    .primary_key(Index::create().col("response_id").col("operation_id"))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_receipt_response")
                            .from("operation_receipts", "response_id")
                            .to("responses", "id")
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for name in ["operation_receipts", "responses", "runs"] {
            manager
                .drop_table(Table::drop().table(name).to_owned())
                .await?;
        }
        Ok(())
    }
}
