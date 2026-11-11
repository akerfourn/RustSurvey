use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("administrator")
                    .col(ColumnDef::new("id").integer().not_null().primary_key())
                    .col(ColumnDef::new("username").string_len(64).not_null())
                    .col(ColumnDef::new("password_hash").string_len(512).not_null())
                    .col(ColumnDef::new("failures").integer().not_null().default(0))
                    .col(
                        ColumnDef::new("blocked_until")
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table("administrator_sessions")
                    .col(
                        ColumnDef::new("token_hash")
                            .string_len(64)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new("administrator_id").integer().not_null())
                    .col(ColumnDef::new("csrf_token").string_len(64).not_null())
                    .col(ColumnDef::new("expires_at").big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_administrator")
                            .from("administrator_sessions", "administrator_id")
                            .to("administrator", "id")
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_session_expiry")
                    .table("administrator_sessions")
                    .col("expires_at")
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table("studies")
                    .col(ColumnDef::new("id").string_len(36).not_null().primary_key())
                    .col(ColumnDef::new("title").string_len(200).not_null())
                    .col(ColumnDef::new("revision").big_integer().not_null())
                    .col(ColumnDef::new("created_at").big_integer().not_null())
                    .col(ColumnDef::new("updated_at").big_integer().not_null())
                    .to_owned(),
            )
            .await?;
        let mut document = ColumnDef::new("document");
        if manager.get_database_backend() == sea_orm_migration::sea_orm::DbBackend::MySql {
            document.custom("LONGTEXT").not_null();
        } else {
            document.text().not_null();
        }
        manager
            .create_table(
                Table::create()
                    .table("definition_revisions")
                    .col(ColumnDef::new("study_id").string_len(36).not_null())
                    .col(ColumnDef::new("revision").big_integer().not_null())
                    .col(&mut document)
                    .col(ColumnDef::new("created_at").big_integer().not_null())
                    .primary_key(Index::create().col("study_id").col("revision"))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_revision_study")
                            .from("definition_revisions", "study_id")
                            .to("studies", "id")
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for name in [
            "definition_revisions",
            "studies",
            "administrator_sessions",
            "administrator",
        ] {
            manager
                .drop_table(Table::drop().table(name).to_owned())
                .await?;
        }
        Ok(())
    }
}
