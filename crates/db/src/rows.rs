//! Rebuilding typed SeaORM values from D1's untyped JSON rows (and formatting
//! values for D1), kept free of the Workers runtime so it is tested natively.
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
        (ColumnKind::DateTime, Json::String(s)) => TIMESTAMP_FORMATS
            .iter()
            .find_map(|format| chrono::NaiveDateTime::parse_from_str(s, format).ok())
            .ok_or_else(bad)?
            .into(),
        (ColumnKind::Text, Json::Null) => Value::String(None),
        (ColumnKind::Text, Json::String(s)) => s.clone().into(),
        _ => return Err(bad()),
    })
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
}
