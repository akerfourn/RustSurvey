use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table("runs")
                    .add_column(
                        ColumnDef::new("participation_policy")
                            .string_len(256)
                            .not_null()
                            .default("{\"mode\":\"open\"}"),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table("invitations")
                    .col(ColumnDef::new("id").string_len(36).not_null().primary_key())
                    .col(ColumnDef::new("run_id").string_len(36).not_null())
                    .col(ColumnDef::new("secret_hash").string_len(64).not_null())
                    .col(ColumnDef::new("max_uses").big_integer())
                    .col(ColumnDef::new("uses").big_integer().not_null().default(0))
                    .col(ColumnDef::new("valid_from").big_integer())
                    .col(ColumnDef::new("valid_until").big_integer())
                    .col(
                        ColumnDef::new("revoked")
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new("response_id").string_len(36))
                    .col(ColumnDef::new("created_at").big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invitation_run")
                            .from("invitations", "run_id")
                            .to("runs", "id"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invitation_response")
                            .from("invitations", "response_id")
                            .to("responses", "id"),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_invitation_secret")
                    .table("invitations")
                    .col("run_id")
                    .col("secret_hash")
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_invitation_response")
                    .table("invitations")
                    .col("response_id")
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_invitation_run")
                    .table("invitations")
                    .col("run_id")
                    .col("created_at")
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("invitations").to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table("runs")
                    .drop_column("participation_policy")
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
