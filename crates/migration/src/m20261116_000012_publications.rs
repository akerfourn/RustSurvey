use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("study_publications")
                    .col(ColumnDef::new("study_id").string_len(36).not_null())
                    .col(ColumnDef::new("revision").big_integer().not_null())
                    .col(ColumnDef::new("title").string_len(200).not_null())
                    .col(ColumnDef::new("published_at").big_integer().not_null())
                    .primary_key(Index::create().col("study_id").col("revision"))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_publication_revision")
                            .from("study_publications", ("study_id", "revision"))
                            .to("definition_revisions", ("study_id", "revision"))
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;
        // Existing questionnaires were already public. Preserve their current public version.
        manager
            .get_connection()
            .execute(
                Query::insert()
                    .into_table("study_publications")
                    .columns(["study_id", "revision", "title", "published_at"])
                    .select_from(
                        Query::select()
                            .columns(["id", "revision", "title", "updated_at"])
                            .from("studies")
                            .to_owned(),
                    )
                    .map_err(|error| DbErr::Custom(error.to_string()))?,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("study_publications").to_owned())
            .await?;
        Ok(())
    }
}
