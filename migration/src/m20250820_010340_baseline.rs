use sea_orm_migration::prelude::*;
use sea_query::Expr;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rooms
        manager
            .create_table(
                Table::create()
                    .table(Room::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Room::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Room::Name).text().not_null())
                    .col(ColumnDef::new(Room::Rent).integer().not_null())
                    .col(ColumnDef::new(Room::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Room::UpdatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .index(Index::create().unique().name("rooms_name_key").col(Room::Name))
                    .to_owned()
            ).await?;

        // Tenants
        manager
            .create_table(
                Table::create()
                    .table(Tenant::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Tenant::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Tenant::RoomId).integer().not_null())
                    .col(ColumnDef::new(Tenant::Name).text().not_null())
                    .col(ColumnDef::new(Tenant::IsActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(Tenant::JoinDate).timestamp().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Tenant::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Tenant::UpdatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .index(Index::create().unique().name("tenants_name_key").col(Tenant::Name))
                    .foreign_key(ForeignKey::create().from(Tenant::Table, Tenant::RoomId).to(Room::Table, Room::Id).on_update(ForeignKeyAction::Cascade).on_delete(ForeignKeyAction::Restrict))
                    .to_owned()
            ).await?;

        // Electricity Readings
        manager
            .create_table(
                Table::create()
                    .table(ElectricityReading::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ElectricityReading::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(ElectricityReading::TenantId).integer().not_null())
                    .col(ColumnDef::new(ElectricityReading::RoomId).integer().not_null())
                    .col(ColumnDef::new(ElectricityReading::PrevReading).integer().not_null())
                    .col(ColumnDef::new(ElectricityReading::CurrReading).integer().not_null())
                    .col(ColumnDef::new(ElectricityReading::Consumption).integer().not_null())
                    .col(ColumnDef::new(ElectricityReading::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(ElectricityReading::UpdatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .foreign_key(ForeignKey::create().from(ElectricityReading::Table, ElectricityReading::TenantId).to(Tenant::Table, Tenant::Id).on_update(ForeignKeyAction::Cascade).on_delete(ForeignKeyAction::Restrict))
                    .foreign_key(ForeignKey::create().from(ElectricityReading::Table, ElectricityReading::RoomId).to(Room::Table, Room::Id).on_update(ForeignKeyAction::Cascade).on_delete(ForeignKeyAction::Restrict))
                    .to_owned()
            ).await?;

        // Bills
        manager
            .create_table(
                Table::create()
                    .table(Bill::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Bill::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Bill::ReadingId).integer().not_null())
                    .col(ColumnDef::new(Bill::TenantId).integer().not_null())
                    .col(ColumnDef::new(Bill::RoomCharges).integer().not_null().default(0))
                    .col(ColumnDef::new(Bill::ElectricCharges).integer().not_null().default(0))
                    .col(ColumnDef::new(Bill::TotalAmount).integer().not_null())
                    .col(ColumnDef::new(Bill::ReceiptUrl).text())
                    .col(ColumnDef::new(Bill::Paid).boolean().not_null().default(false))
                    .col(ColumnDef::new(Bill::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Bill::UpdatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .index(Index::create().unique().name("bills_readingId_key").col(Bill::ReadingId))
                    .foreign_key(ForeignKey::create().from(Bill::Table, Bill::ReadingId).to(ElectricityReading::Table, ElectricityReading::Id).on_update(ForeignKeyAction::Cascade).on_delete(ForeignKeyAction::Restrict))
                    .foreign_key(ForeignKey::create().from(Bill::Table, Bill::TenantId).to(Tenant::Table, Tenant::Id).on_update(ForeignKeyAction::Cascade).on_delete(ForeignKeyAction::Restrict))
                    .to_owned()
            ).await?;

        // Additional Charges
        manager
            .create_table(
                Table::create()
                    .table(AdditionalCharge::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AdditionalCharge::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(AdditionalCharge::BillId).integer().not_null())
                    .col(ColumnDef::new(AdditionalCharge::Amount).integer().not_null().default(0))
                    .col(ColumnDef::new(AdditionalCharge::Description).text().not_null())
                    .col(ColumnDef::new(AdditionalCharge::CreatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(AdditionalCharge::UpdatedAt).timestamp().not_null().default(Expr::current_timestamp()))
                    .foreign_key(ForeignKey::create().from(AdditionalCharge::Table, AdditionalCharge::BillId).to(Bill::Table, Bill::Id).on_update(ForeignKeyAction::Cascade).on_delete(ForeignKeyAction::Cascade))
                    .to_owned()
            ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse dependency order
        manager.drop_table(Table::drop().table(AdditionalCharge::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Bill::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ElectricityReading::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Tenant::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Room::Table).to_owned()).await?;
        Ok(())
    }
}

// ================== Identifier enums ==================
#[derive(DeriveIden)]
enum Room { Table, Id, Name, Rent, CreatedAt, UpdatedAt }

#[derive(DeriveIden)]
enum Tenant { Table, Id, RoomId, Name, IsActive, JoinDate, CreatedAt, UpdatedAt }

#[derive(DeriveIden)]
enum ElectricityReading { Table, Id, TenantId, RoomId, PrevReading, CurrReading, Consumption, CreatedAt, UpdatedAt }

#[derive(DeriveIden)]
enum Bill { Table, Id, ReadingId, TenantId, RoomCharges, ElectricCharges, TotalAmount, ReceiptUrl, Paid, CreatedAt, UpdatedAt }

#[derive(DeriveIden)]
enum AdditionalCharge { Table, Id, BillId, Amount, Description, CreatedAt, UpdatedAt }
