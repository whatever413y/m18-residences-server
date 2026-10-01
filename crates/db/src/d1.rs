//! SeaORM on Cloudflare D1. SeaORM has no D1 driver, so this implements its
//! `proxy` connection: SeaORM hands over SQL + values, D1 runs them, and the
//! untyped JSON rows are rebuilt into typed values with [`column_kinds`].
//!
//! Through the proxy, SeaORM reads a value of the wrong type in an `Option`
//! field as `None` without an error, so the rebuild must be exact; unknown
//! columns (aliases, aggregates) fall back to the JSON type.
use std::{collections::BTreeMap, collections::HashMap, sync::Arc};

use sea_orm::{
    Database, DbBackend, DbErr, ProxyDatabaseTrait, ProxyExecResult, ProxyRow, RuntimeErr,
    Statement, Value,
};
use serde_json::{Map, Value as Json};
use worker::{D1Database, D1PreparedStatement, send::SendFuture, wasm_bindgen::JsValue};

use crate::{
    Atomic, Db,
    rows::{format_timestamp, inferred, typed, unaliased},
    schema::{ColumnKind, column_kinds},
};

/// A [`Db`] over the D1 binding.
pub async fn connect(binding: D1Database) -> Result<Db, DbErr> {
    let d1 = Arc::new(D1 {
        db: binding,
        kinds: column_kinds().map_err(DbErr::Custom)?,
    });
    let proxy: Arc<Box<dyn ProxyDatabaseTrait>> = Arc::new(Box::new(Proxy(d1.clone())));
    let conn = Database::connect_proxy(DbBackend::Sqlite, proxy).await?;
    Ok(Db::new(conn, d1))
}

#[derive(Debug)]
struct D1 {
    db: D1Database,
    kinds: HashMap<&'static str, ColumnKind>,
}

impl D1 {
    fn prepare(&self, statement: &Statement) -> Result<D1PreparedStatement, DbErr> {
        let prepared = self.db.prepare(statement.sql.clone());
        let Some(values) = &statement.values else {
            return Ok(prepared);
        };
        if values.0.is_empty() {
            return Ok(prepared);
        }
        let values = values.0.iter().map(to_js).collect::<Result<Vec<_>, _>>()?;
        prepared.bind(&values).map_err(d1_err)
    }

    fn row(&self, columns: Map<String, Json>) -> Result<ProxyRow, DbErr> {
        let mut values = BTreeMap::new();
        for (name, json) in columns {
            let kind = self
                .kinds
                .get(name.as_str())
                .or_else(|| unaliased(&name).and_then(|n| self.kinds.get(n)));
            let value = match kind {
                Some(kind) => {
                    typed(*kind, &json).map_err(|e| DbErr::Type(format!("column `{name}`: {e}")))?
                }
                None => inferred(&json),
            };
            values.insert(name, value);
        }
        Ok(ProxyRow::new(values))
    }
}

#[async_trait::async_trait]
impl Atomic for D1 {
    async fn run(&self, statements: Vec<Statement>) -> Result<(), DbErr> {
        let prepared = statements
            .iter()
            .map(|s| self.prepare(s))
            .collect::<Result<Vec<_>, _>>()?;
        SendFuture::new(self.db.batch(prepared))
            .await
            .map_err(d1_err)?;
        Ok(())
    }
}

#[derive(Debug)]
struct Proxy(Arc<D1>);

#[async_trait::async_trait]
impl ProxyDatabaseTrait for Proxy {
    async fn query(&self, statement: Statement) -> Result<Vec<ProxyRow>, DbErr> {
        let prepared = self.0.prepare(&statement)?;
        let result = SendFuture::new(async move { prepared.all().await })
            .await
            .map_err(d1_err)?;
        let rows: Vec<Map<String, Json>> = result.results().map_err(d1_err)?;
        rows.into_iter().map(|row| self.0.row(row)).collect()
    }

    async fn execute(&self, statement: Statement) -> Result<ProxyExecResult, DbErr> {
        let prepared = self.0.prepare(&statement)?;
        let result = SendFuture::new(async move { prepared.run().await })
            .await
            .map_err(d1_err)?;
        let meta = result
            .meta()
            .map_err(d1_err)?
            .ok_or_else(|| DbErr::Custom("D1 returned no result metadata".into()))?;
        let last_insert_id = meta.last_row_id.unwrap_or(0).max(0) as u64;
        Ok(ProxyExecResult::new(
            last_insert_id,
            meta.changes.unwrap_or(0) as u64,
        ))
    }

    async fn begin(&self) {
        // SeaORM gives this hook no way to fail; refuse loudly instead of silently not being atomic.
        panic!("D1 has no interactive transactions: use Db::atomic");
    }
}

fn to_js(value: &Value) -> Result<JsValue, DbErr> {
    let number = |n: f64| JsValue::from_f64(n);
    Ok(match value {
        Value::Bool(Some(b)) => number(if *b { 1.0 } else { 0.0 }),
        Value::TinyInt(Some(n)) => number(*n as f64),
        Value::SmallInt(Some(n)) => number(*n as f64),
        Value::Int(Some(n)) => number(*n as f64),
        Value::BigInt(Some(n)) => number(*n as f64),
        Value::TinyUnsigned(Some(n)) => number(*n as f64),
        Value::SmallUnsigned(Some(n)) => number(*n as f64),
        Value::Unsigned(Some(n)) => number(*n as f64),
        Value::BigUnsigned(Some(n)) => number(*n as f64),
        Value::Float(Some(n)) => number(*n as f64),
        Value::Double(Some(n)) => number(*n),
        Value::String(Some(s)) => JsValue::from_str(s),
        Value::ChronoDateTime(Some(dt)) => JsValue::from_str(&format_timestamp(dt)),
        Value::Bool(None)
        | Value::TinyInt(None)
        | Value::SmallInt(None)
        | Value::Int(None)
        | Value::BigInt(None)
        | Value::TinyUnsigned(None)
        | Value::SmallUnsigned(None)
        | Value::Unsigned(None)
        | Value::BigUnsigned(None)
        | Value::Float(None)
        | Value::Double(None)
        | Value::String(None)
        | Value::ChronoDateTime(None) => JsValue::NULL,
        other => return Err(DbErr::Type(format!("D1 adapter cannot bind {other:?}"))),
    })
}

fn d1_err(err: worker::Error) -> DbErr {
    DbErr::Query(RuntimeErr::Internal(format!("D1: {err}")))
}
