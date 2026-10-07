pub mod additional_charge_repo;
pub mod bill_repo;
pub mod payment_method_repo;
pub mod reading_read_repo;
pub mod tenant_read_repo;

/// D1 accepts at most 100 bound parameters per query, so `IN (...)` lists are
/// split into chunks of this size.
pub const MAX_IDS_PER_QUERY: usize = 100;
