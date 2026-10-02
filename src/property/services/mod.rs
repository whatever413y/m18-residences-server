pub mod electricity_reading_service;
pub mod room_service;
pub mod tenant_service;

use sea_orm::DbErr;

/// Which constraint a failed write broke. SQLite (D1 and the native tests)
/// reports constraint failures only in the message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    Unique,
    ForeignKey,
}

pub fn violation(err: &DbErr) -> Option<Violation> {
    let text = err.to_string();
    if text.contains("UNIQUE constraint failed") {
        Some(Violation::Unique)
    } else if text.contains("FOREIGN KEY constraint failed") {
        Some(Violation::ForeignKey)
    } else {
        None
    }
}
