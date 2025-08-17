use sea_orm::ActiveValue;

fn value_or_zero(v: ActiveValue<i32>) -> i32 {
    if let ActiveValue::Set(x) = v { x } else { 0 }
}

pub fn calculate_consumption(prev: ActiveValue<i32>, curr: ActiveValue<i32>) -> i32 {
    value_or_zero(curr) - value_or_zero(prev)
}