// Postgres types
#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "billing_type", rename_all = "snake_case")]
pub enum BillingType {
    #[default]
    PerKwh,
    PerMinute,
}

#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "unit_of_length", rename_all = "lowercase")]
pub enum UnitOfLength {
    Km,
    #[default]
    Mi,
}

#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "unit_of_pressure", rename_all = "lowercase")]
pub enum UnitOfPressure {
    Bar,
    #[default]
    Psi,
}

#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "unit_of_temperature")]
pub enum UnitOfTemperature {
    C,
    #[default]
    F,
}

#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "range", rename_all = "lowercase")]
pub enum Range {
    Ideal,
    #[default]
    Rated,
}

#[derive(Debug, PartialEq, Clone, Default, sqlx::Type)]
#[sqlx(type_name = "charge_stat", rename_all = "snake_case")]
pub enum ChargeStat {
    #[default]
    Start,
    Charging,
    Done,
}