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
    Start,
    Stop,
    Charging,
    Done,
    #[default]
    Unknown,
}

#[derive(Debug, PartialEq, Clone, Default, sqlx::Type)]
#[sqlx(type_name = "drive_stat", rename_all = "snake_case")]
pub enum DriveStatus {
    Start,
    /// Start a new drive
    Driving,
    /// Currently driving, record the position/drive statistics data
    Stop,
    /// Stop the current drive
    NotDriving,
    /// Not driving, waiting for a drive to start
    Restart,
    /// Stop the current drive and immediately start a new one.
    /// Use the previous data point as the last data point for the previous drive
    /// and use the current data point as the starting of a new drive (Leave the end_date None to
    /// mark we don't know when the current drive ended).
    /// Leave the start_date None to mark we don't know when the drive was started.
    #[default]
    Unknown, // Unknown state, do nothing
}