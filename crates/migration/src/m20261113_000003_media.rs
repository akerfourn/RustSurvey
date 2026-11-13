use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut body = ColumnDef::new("body");
        if manager.get_database_backend() == sea_orm_migration::sea_orm::DbBackend::MySql {
            body.custom("LONGBLOB").not_null();
        } else {
            body.binary().not_null();
        }
        manager
            .create_table(
                Table::create()
                    .table("objects")
                    .col(
                        ColumnDef::new("object_key")
                            .string_len(64)
                            .not_null()
                            .primary_key(),
                    )
                    .col(&mut body)
                    .col(ColumnDef::new("size").big_integer().not_null())
                    .col(ColumnDef::new("sha256").string_len(64).not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table("media")
                    .col(ColumnDef::new("id").string_len(36).not_null().primary_key())
                    .col(ColumnDef::new("study_id").string_len(36).not_null())
                    .col(
                        ColumnDef::new("object_key")
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new("filename").string_len(1024).not_null())
                    .col(ColumnDef::new("content_type").string_len(64).not_null())
                    .col(ColumnDef::new("size").big_integer().not_null())
                    .col(ColumnDef::new("sha256").string_len(64).not_null())
                    .col(ColumnDef::new("state").string_len(16).not_null())
                    .col(ColumnDef::new("created_at").big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_media_study")
                            .from("media", "study_id")
                            .to("studies", "id")
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_media_study")
                    .table("media")
                    .col("study_id")
                    .col("created_at")
                    .col("id")
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table("media_references")
                    .col(ColumnDef::new("study_id").string_len(36).not_null())
                    .col(ColumnDef::new("revision").big_integer().not_null())
                    .col(ColumnDef::new("media_id").string_len(36).not_null())
                    .primary_key(
                        Index::create()
                            .col("study_id")
                            .col("revision")
                            .col("media_id"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_media_reference_revision")
                            .from("media_references", ("study_id", "revision"))
                            .to("definition_revisions", ("study_id", "revision"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_media_reference_object")
                            .from("media_references", "media_id")
                            .to("media", "id")
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in ["media_references", "media", "objects"] {
            manager
                .drop_table(Table::drop().table(table).to_owned())
                .await?;
        }
        Ok(())
    }
}
