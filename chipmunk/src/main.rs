#![feature(async_closure)]
#![feature(let_chains)]
#![feature(stmt_expr_attributes)]

use chipmunk::{
    config::{load_env_vars, Config},
    database::{self, tables::token::Token},
};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional argument to operate on
    option: Option<String>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    debug: bool,

    /// Store tesla auth token in database
    #[arg(short, long, action = clap::ArgAction::Set)]
    token: Option<String>,

    /// How many row to fetch from car_data when running `convertdb`.
    /// Use 0 to fetch all data.
    #[arg(short, long, default_value_t = 50_000, action = clap::ArgAction::Set)]
    num_rows: i64,
}

macro_rules! print_err_and_exit {
    () => {
        |e| {
            log::error!("{e}");
            std::process::exit(1);
        }
    };
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    std::env::set_var("RUST_BACKTRACE", "1"); // Enable backtrace
    chipmunk::init_log();

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
        log::error!("Error configuring tracing: {e}");
    }

    // console_subscriber::init();

    let env = load_env_vars().unwrap_or_else(print_err_and_exit!());

    log::info!("Initializing database");
    let pool = database::initialize(&env.database_url)
        .await
        .unwrap_or_else(print_err_and_exit!());
    log::info!("Database initialized");

    let mut config = Config::new(&pool).await;

    let cli = Cli::parse();

    // If a token is provided, store it in the database
    if let Some(refresh_token) = cli.token {
        match tesla_api::auth::refresh_access_token(refresh_token.as_str()).await {
            Ok(tokens) => Token::db_insert(&pool, &tokens, env.encryption_key.as_str()).await?,
            Err(e) => log::error!("{e}"),
        };
    }

    if let Some(option) = cli.option.as_deref() {
        match option {
            "tasks" => chipmunk::tasks::run(&pool, &mut config)
                .await
                .unwrap_or_else(print_err_and_exit!()),
            "convertdb" => chipmunk::tasks::convert_db(&pool, &config, cli.num_rows)
                .await
                .unwrap_or_else(print_err_and_exit!()),
            unknown => log::error!("Unknown command line argument `{unknown}`"),
        };
    }

    tracing::info!("exiting");

    Ok(())
}
