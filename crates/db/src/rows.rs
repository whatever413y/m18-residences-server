//! Rebuilding typed SeaORM values from D1's untyped JSON rows (and formatting
//! values for D1), kept free of the Workers runtime so it is tested natively.
use std::collections::{BTreeMap, HashMap};

use sea_orm::Value;
use serde_json::Value as Json;

use crate::schema::ColumnKind;

/// Formats D1 timestamps may come in: `CURRENT_TIMESTAMP` and [`format_timestamp`].
const TIMESTAMP_FORMATS: [&str; 2] = ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"];

/// How timestamps are written to D1: the same shape as `CURRENT_TIMESTAMP`, plus fractional seconds when set.
pub fn format_timestamp(dt: &chrono::NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S%.f").to_string()
}

/// `A_name` / `B_name` aliases SeaORM gives joined columns.
pub fn unaliased(name: &str) -> Option<&str> {
    name.strip_prefix("A_").or_else(|| name.strip_prefix("B_"))
}

/// The value of a known column, exactly typed.
pub fn typed(kind: ColumnKind, json: &Json) -> Result<Value, String> {
    let bad = || format!("cannot read {json} as {kind:?}");
    Ok(match (kind, json) {
        (ColumnKind::Int, Json::Null) => Value::Int(None),
        (ColumnKind::Int, Json::Number(n)) => {
            let n = n.as_i64().ok_or_else(bad)?;
            Value::Int(Some(i32::try_from(n).map_err(|_| bad())?))
        }
        (ColumnKind::Bool, Json::Null) => Value::Bool(None),
        (ColumnKind::Bool, Json::Number(n)) => Value::Bool(Some(n.as_i64().ok_or_else(bad)? != 0)),
        (ColumnKind::Bool, Json::Bool(b)) => Value::Bool(Some(*b)),
        (ColumnKind::DateTime, Json::Null) => Option::<chrono::NaiveDateTime>::None.into(),
        (ColumnKind::DateTime, Json::String(s)) => parse_timestamp(s).ok_or_else(bad)?.into(),
        (ColumnKind::Text, Json::Null) => Value::String(None),
        (ColumnKind::Text, Json::String(s)) => s.clone().into(),
        _ => return Err(bad()),
    })
}

/// A D1 timestamp: `YYYY-MM-DD HH:MM:SS` (or with `T`), optionally with up to 9 fraction digits. Parsed by hand,
/// because this runs for every timestamp of every row and chrono's format-string parser costs several times more;
/// anything unusual falls back to [`TIMESTAMP_FORMATS`].
pub fn parse_timestamp(s: &str) -> Option<chrono::NaiveDateTime> {
    fast_timestamp(s).or_else(|| {
        TIMESTAMP_FORMATS
            .iter()
            .find_map(|format| chrono::NaiveDateTime::parse_from_str(s, format).ok())
    })
}

fn fast_timestamp(s: &str) -> Option<chrono::NaiveDateTime> {
    let b = s.as_bytes();
    if b.len() < 19
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b' ' | b'T')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let digits = |from: usize, to: usize| -> Option<u32> {
        b[from..to].iter().try_fold(0u32, |n, &c| {
            c.is_ascii_digit().then(|| n * 10 + u32::from(c - b'0'))
        })
    };
    let nanos = match &b[19..] {
        [] => 0,
        [b'.', fraction @ ..] if (1..=9).contains(&fraction.len()) => {
            digits(20, b.len())? * 10u32.pow(9 - fraction.len() as u32)
        }
        _ => return None,
    };
    chrono::NaiveDate::from_ymd_opt(digits(0, 4)? as i32, digits(5, 7)?, digits(8, 10)?)?
        .and_hms_nano_opt(digits(11, 13)?, digits(14, 16)?, digits(17, 19)?, nanos)
}

/// One row as D1 returns it (a JSON object), with its fields kept in order and without building a map: every row of
/// a result has the same columns in the same order, which [`typed_object_rows`] relies on.
#[derive(Debug, Default)]
pub struct ObjectRow(pub Vec<(String, Json)>);

impl<'de> serde::Deserialize<'de> for ObjectRow {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> serde::de::Visitor<'de> for Fields {
            type Value = ObjectRow;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a row object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<ObjectRow, A::Error> {
                let mut fields = Vec::with_capacity(map.size_hint().unwrap_or(8));
                while let Some(field) = map.next_entry()? {
                    fields.push(field);
                }
                Ok(ObjectRow(fields))
            }
        }
        deserializer.deserialize_map(Fields)
    }
}

/// D1's row objects rebuilt into typed values: the column names (and so their kinds) come from the first row and
/// every other row must have the same columns in the same order.
pub fn typed_object_rows(
    rows: Vec<ObjectRow>,
    kinds: &HashMap<&'static str, ColumnKind>,
) -> Result<Vec<BTreeMap<String, Value>>, String> {
    let Some(first) = rows.first() else {
        return Ok(Vec::new());
    };
    let header: Vec<Json> = first
        .0
        .iter()
        .map(|(name, _)| Json::String(name.clone()))
        .collect();
    let mut table = Vec::with_capacity(rows.len() + 1);
    table.push(header);
    for row in rows {
        if row.0.len() != table[0].len()
            || row
                .0
                .iter()
                .zip(&table[0])
                .any(|((name, _), h)| h.as_str() != Some(name))
        {
            return Err("rows of one result have different columns".into());
        }
        table.push(row.0.into_iter().map(|(_, value)| value).collect());
    }
    typed_rows(table, kinds)
}

/// A table (the column names, then one array of values per row) rebuilt into typed values. Each column's kind is
/// looked up once per query, not once per value: a known column (directly or through its `A_`/`B_` alias) is typed
/// exactly, an unknown one (aggregate, alias) by its JSON type.
pub fn typed_rows(
    mut table: Vec<Vec<Json>>,
    kinds: &HashMap<&'static str, ColumnKind>,
) -> Result<Vec<BTreeMap<String, Value>>, String> {
    if table.is_empty() {
        return Ok(Vec::new());
    }
    let header = table.remove(0);
    let columns = header
        .into_iter()
        .map(|name| match name {
            Json::String(name) => {
                let kind = kinds
                    .get(name.as_str())
                    .or_else(|| unaliased(&name).and_then(|n| kinds.get(n)))
                    .copied();
                Ok((name, kind))
            }
            other => Err(format!("column name {other} is not a string")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    table
        .into_iter()
        .map(|row| {
            if row.len() != columns.len() {
                return Err(format!(
                    "row has {} values for {} columns",
                    row.len(),
                    columns.len()
                ));
            }
            columns
                .iter()
                .zip(row)
                .map(|((name, kind), json)| {
                    let value = match kind {
                        Some(kind) => {
                            typed(*kind, &json).map_err(|e| format!("column `{name}`: {e}"))?
                        }
                        None => inferred(&json),
                    };
                    Ok((name.clone(), value))
                })
                .collect()
        })
        .collect()
}

/// The value of a column the schema doesn't know (an aggregate or alias), by its JSON type.
pub fn inferred(json: &Json) -> Value {
    match json {
        Json::Null => Value::String(None),
        Json::Bool(b) => Value::Bool(Some(*b)),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Value::BigInt(Some(i)),
            None => Value::Double(n.as_f64()),
        },
        Json::String(s) => s.clone().into(),
        other => other.to_string().into(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::*;

    fn at(h: u32, m: u32, s: u32) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 1)
            .unwrap()
            .and_hms_opt(h, m, s)
            .unwrap()
    }

    #[test]
    fn rebuilds_each_kind() {
        assert_eq!(
            typed(ColumnKind::Int, &json!(5000)).unwrap(),
            Value::Int(Some(5000))
        );
        assert_eq!(
            typed(ColumnKind::Int, &json!(-7)).unwrap(),
            Value::Int(Some(-7))
        );
        assert_eq!(
            typed(ColumnKind::Bool, &json!(1)).unwrap(),
            Value::Bool(Some(true))
        );
        assert_eq!(
            typed(ColumnKind::Bool, &json!(0)).unwrap(),
            Value::Bool(Some(false))
        );
        assert_eq!(
            typed(ColumnKind::Text, &json!("Room 101")).unwrap(),
            Value::String(Some("Room 101".into()))
        );
        assert_eq!(
            typed(ColumnKind::DateTime, &json!("2026-10-01 09:16:40")).unwrap(),
            at(9, 16, 40).into()
        );
    }

    #[test]
    fn nulls_keep_their_kind() {
        assert_eq!(
            typed(ColumnKind::Text, &Json::Null).unwrap(),
            Value::String(None)
        );
        assert_eq!(
            typed(ColumnKind::Int, &Json::Null).unwrap(),
            Value::Int(None)
        );
        assert_eq!(
            typed(ColumnKind::DateTime, &Json::Null).unwrap(),
            Option::<chrono::NaiveDateTime>::None.into()
        );
    }

    #[test]
    fn timestamps_round_trip_with_and_without_fractions() {
        let whole = at(9, 16, 40);
        assert_eq!(format_timestamp(&whole), "2026-10-01 09:16:40");
        let fraction = whole + chrono::Duration::microseconds(123_456);
        assert_eq!(
            typed(ColumnKind::DateTime, &json!(format_timestamp(&fraction))).unwrap(),
            fraction.into()
        );
        assert_eq!(
            typed(ColumnKind::DateTime, &json!("2026-10-01T09:16:40")).unwrap(),
            whole.into()
        );
    }

    #[test]
    fn refuses_values_of_the_wrong_type() {
        assert!(typed(ColumnKind::Int, &json!("12")).is_err());
        assert!(
            typed(ColumnKind::Int, &json!(2_147_483_648_i64)).is_err(),
            "beyond i32"
        );
        assert!(typed(ColumnKind::Int, &json!(1.5)).is_err());
        assert!(typed(ColumnKind::DateTime, &json!("yesterday")).is_err());
        assert!(typed(ColumnKind::Text, &json!(3)).is_err());
    }

    #[test]
    fn infers_unknown_columns_and_strips_join_aliases() {
        assert_eq!(inferred(&json!(3)), Value::BigInt(Some(3)));
        assert_eq!(inferred(&json!("x")), Value::String(Some("x".into())));
        assert_eq!(unaliased("A_created_at"), Some("created_at"));
        assert_eq!(unaliased("num_items"), None);
    }

    #[test]
    fn the_fast_timestamp_parser_agrees_with_chrono() {
        for text in [
            "2026-10-01 09:16:40",
            "2026-10-01T09:16:40",
            "2026-10-01 09:16:40.5",
            "2026-10-01 09:16:40.123456",
            "2026-10-01 09:16:40.123456789",
            "2024-02-29 23:59:59",
            "0001-01-01 00:00:00",
        ] {
            let expected = TIMESTAMP_FORMATS
                .iter()
                .find_map(|f| chrono::NaiveDateTime::parse_from_str(text, f).ok());
            assert_eq!(fast_timestamp(text), expected, "{text}");
            assert_eq!(parse_timestamp(text), expected, "{text}");
        }
        for bad in [
            "2026-02-30 00:00:00",
            "2026-10-01 24:00:00",
            "2026-10-01 09:16",
            "2026-10-01 09:16:40.",
            "2026-10-01 09:16:4x",
            "2026/10/01 09:16:40",
            "",
        ] {
            assert_eq!(parse_timestamp(bad), None, "{bad}");
        }
    }

    #[test]
    fn rebuilds_raw_rows_with_the_column_kinds() {
        let kinds: HashMap<&'static str, ColumnKind> = [
            ("id", ColumnKind::Int),
            ("paid", ColumnKind::Bool),
            ("created_at", ColumnKind::DateTime),
        ]
        .into_iter()
        .collect();
        let table = vec![
            vec![
                json!("id"),
                json!("A_paid"),
                json!("created_at"),
                json!("total"),
            ],
            vec![json!(1), json!(1), json!("2026-10-01 09:16:40"), json!(42)],
            vec![json!(2), json!(0), Json::Null, json!(7)],
        ];
        let rows = typed_rows(table, &kinds).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["id"], Value::Int(Some(1)));
        assert_eq!(
            rows[0]["A_paid"],
            Value::Bool(Some(true)),
            "alias typed like its column"
        );
        assert_eq!(rows[0]["created_at"], at(9, 16, 40).into());
        assert_eq!(
            rows[0]["total"],
            Value::BigInt(Some(42)),
            "unknown column inferred"
        );
        assert_eq!(
            rows[1]["created_at"],
            Option::<chrono::NaiveDateTime>::None.into()
        );
        assert!(typed_rows(Vec::new(), &kinds).unwrap().is_empty());
        assert!(
            typed_rows(vec![vec![json!("id")]], &kinds)
                .unwrap()
                .is_empty(),
            "header only"
        );
        assert!(
            typed_rows(vec![vec![json!("id")], vec![json!(1), json!(2)]], &kinds).is_err(),
            "ragged row"
        );
        assert!(
            typed_rows(vec![vec![json!("id")], vec![json!("x")]], &kinds).is_err(),
            "mistyped value"
        );
    }

    #[test]
    fn rebuilds_object_rows_in_column_order() {
        let kinds: HashMap<&'static str, ColumnKind> =
            [("id", ColumnKind::Int), ("name", ColumnKind::Text)]
                .into_iter()
                .collect();
        let rows: Vec<ObjectRow> = serde_json::from_str(
            r#"[{"id":1,"name":"Room 1","count":3},{"id":2,"name":null,"count":4}]"#,
        )
        .unwrap();
        let typed = typed_object_rows(rows, &kinds).unwrap();
        assert_eq!(typed[0]["id"], Value::Int(Some(1)));
        assert_eq!(typed[0]["name"], Value::String(Some("Room 1".into())));
        assert_eq!(typed[0]["count"], Value::BigInt(Some(3)));
        assert_eq!(typed[1]["name"], Value::String(None));
        assert!(typed_object_rows(Vec::new(), &kinds).unwrap().is_empty());
        let mixed: Vec<ObjectRow> = serde_json::from_str(r#"[{"id":1},{"name":"x"}]"#).unwrap();
        assert!(
            typed_object_rows(mixed, &kinds).is_err(),
            "rows with other columns are refused, not misread"
        );
    }
}
