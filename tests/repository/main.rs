//! Repository integration tests, compiled as one test binary. They share the
//! `m18_test` database and TRUNCATE tables, so `.cargo/config.toml` makes them
//! run one at a time.

#[path = "../common/mod.rs"]
mod common;

mod room;
mod tenant;
mod electricity_reading;
mod bill;
mod additional_charge;
