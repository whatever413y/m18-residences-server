//! What the code knows about the schema at runtime: the migration files, and
//! the type of every column (D1 returns rows as untyped JSON).
use std::collections::HashMap;

use sea_orm::{ColumnTrait, ColumnType, EntityTrait, IdenStatic, Iterable};

use crate::entities::{additional_charge, bill, electricity_reading, room, tenant};

/// The D1 migrations, in order. Must list every file in `migrations/` (a test checks it).
pub const MIGRATIONS: &[(&str, &str)] = &[(
    "0001_baseline.sql",
    include_str!("../migrations/0001_baseline.sql"),
)];

/// How a column's value is stored in SQLite and rebuilt into a typed value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnKind {
    Int,
    Bool,
    DateTime,
    Text,
}

impl ColumnKind {
    pub fn of(column_type: &ColumnType) -> Self {
        match column_type {
            ColumnType::Integer
            | ColumnType::SmallInteger
            | ColumnType::TinyInteger
            | ColumnType::BigInteger => Self::Int,
            ColumnType::Boolean => Self::Bool,
            ColumnType::DateTime | ColumnType::Timestamp => Self::DateTime,
            _ => Self::Text,
        }
    }
}

/// Column name → kind across every entity. Fails if two tables give one column
/// name different kinds, since D1 rows only carry the column name.
pub fn column_kinds() -> Result<HashMap<&'static str, ColumnKind>, String> {
    let mut kinds = HashMap::new();
    add::<room::Entity>(&mut kinds)?;
    add::<tenant::Entity>(&mut kinds)?;
    add::<electricity_reading::Entity>(&mut kinds)?;
    add::<bill::Entity>(&mut kinds)?;
    add::<additional_charge::Entity>(&mut kinds)?;
    Ok(kinds)
}

fn add<E: EntityTrait>(kinds: &mut HashMap<&'static str, ColumnKind>) -> Result<(), String> {
    for column in E::Column::iter() {
        let kind = ColumnKind::of(column.def().get_column_type());
        if let Some(previous) = kinds.insert(column.as_str(), kind)
            && previous != kind
        {
            return Err(format!(
                "column `{}` is {previous:?} in one table and {kind:?} in another",
                column.as_str()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_column_has_one_kind() {
        let kinds = column_kinds().expect("conflicting column kinds");
        assert_eq!(kinds["id"], ColumnKind::Int);
        assert_eq!(kinds["paid"], ColumnKind::Bool);
        assert_eq!(kinds["is_active"], ColumnKind::Bool);
        assert_eq!(kinds["join_date"], ColumnKind::DateTime);
        assert_eq!(kinds["created_at"], ColumnKind::DateTime);
        assert_eq!(kinds["receipt_url"], ColumnKind::Text);
        assert_eq!(kinds["total_amount"], ColumnKind::Int);
    }

    #[test]
    fn migrations_list_matches_the_directory() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.ends_with(".sql"))
            .collect();
        files.sort();
        let listed: Vec<&str> = MIGRATIONS.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            files, listed,
            "update schema::MIGRATIONS when adding a migration"
        );
    }
}
