#![feature(async_closure)]
#![feature(let_chains)]

use std::{io::Write, thread, time};

use anyhow::Context;
use backend::{
    get_default_wsmsg,
    server::{MpscTopic, TeslaServer},
};
use chipmunk::environment::Environment;
use chipmunk::{
    database::{self, tables::convert_database},
    environment, logger,
};
use clap::Parser;
use sqlx::PgPool;
use tokio::sync::mpsc;

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
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    std::env::set_var("RUST_BACKTRACE", "1"); // Enable backtrace
    init_log();

    let env = environment::load().unwrap_or_else(print_err_and_exit!());

    log::info!("Initializing database {}", env.database_url);
    let pool = database::initialize(&env.database_url)
        .await
        .unwrap_or_else(print_err_and_exit!());
    log::info!("Database initialized");

    let cli = Cli::parse();

    // If token is provided, store it in the database
    if let Some(refresh_token) = cli.token {
        match tesla_api::auth::refresh_access_token(refresh_token.as_str()).await {
            Ok(tokens) => {
                database::token::insert(&pool, tokens, env.encryption_key.as_str()).await?
            }
            Err(e) => log::error!("{e}"),
        };
    }

    if let Some(option) = cli.option.as_deref() {
        match option {
            "convertdb" => {
                let car_data_database_url = std::env::var("CAR_DATA_DATABASE_URL")
                    .context("Cannot read CAR_DATA_DATABASE_URL")?;

                let car_data_pool = database::initilize_car_data(&car_data_database_url).await?;
                convert_database(&pool, &car_data_pool, cli.num_rows).await?;
            }
            "ws" => {
                let (tx, _rx) = mpsc::unbounded_channel();
                let server = TeslaServer::start(env.http_port, tx);
                let mut counter = 0;
                loop {
                    server
                        .lock()
                        .unwrap()
                        .broadcast(get_default_wsmsg(counter))
                        .await;
                    counter += 1;
                    thread::sleep(time::Duration::from_secs(1));
                }
            }
            "log" => log(&pool, &env).await?,
            unknown => eprintln!("Unknown option `{unknown}`"),
        };
    }

    Ok(())
}

async fn log(pool: &PgPool, env: &Environment) -> anyhow::Result<()> {
    let (server_tx, mut server_rx) = mpsc::unbounded_channel();
    let server = TeslaServer::start(env.http_port, server_tx);

    let (logger_tx, logger_rx) = mpsc::unbounded_channel();

    // Make copies so that we can move these into the future without causing borrow errors
    let encryption_key = env.encryption_key.clone();
    let pool1 = pool.clone();
    let pool2 = pool.clone();

    let cmd_handler = tokio::task::spawn(async move {
        while let Some(topic) = server_rx.recv().await {
            match topic {
                MpscTopic::Logging(value) => {
                    if let Err(e) = logger_tx.send(value) {
                        log::error!("{e}");
                    }
                }
                MpscTopic::RefreshToken(refresh_token) => {
                    let tokens =
                        match tesla_api::auth::refresh_access_token(refresh_token.as_str()).await {
                            Ok(t) => t,
                            Err(e) => {
                                log::error!("{e}");
                                continue;
                            }
                        };
                    if let Err(e) =
                        database::token::insert(&pool1, tokens, encryption_key.as_str()).await
                    {
                        log::error!("{e}");
                    }
                }
            }
        }
    });

    let server_clone = server.clone();
    let status_reporter = thread::spawn(move || {
        futures::executor::block_on(async {
            loop {
                match server_clone.lock() {
                    Ok(srv) => {
                        let msg = srv.get_status_str();
                        srv.broadcast(msg).await;
                    }
                    Err(e) => log::error!("Error getting server lock: {e}"),
                }
                thread::sleep(time::Duration::from_secs(1));
            }
        });
    });

    tokio::select! {
        res = cmd_handler => res?,
        res = logger::start(&pool2, server, &env.encryption_key, logger_rx) => res?,
    }

    // Panic if any of the threads/tasks return error
    status_reporter.join().unwrap();

    Ok(())
}

fn get_file_name(path_str: Option<&str>) -> String {
    if let Some(path_str_val) = path_str {
        let path = std::path::Path::new(path_str_val);
        if let Some(file_name) = path.file_name() {
            if let Some(s) = file_name.to_str() {
                return s.to_string();
            }
        }
    }

    "unknown".to_string()
}

fn init_log() {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            let level = record.level();
            let mut style = buf.style();
            match record.level() {
                log::Level::Error => style.set_color(env_logger::fmt::Color::Red),
                log::Level::Warn => style.set_color(env_logger::fmt::Color::Yellow),
                log::Level::Info => style.set_color(env_logger::fmt::Color::Green),
                log::Level::Debug => style.set_color(env_logger::fmt::Color::Blue),
                log::Level::Trace => style.set_color(env_logger::fmt::Color::Cyan),
            };
            let mut target_style = buf.style();
            target_style.set_color(env_logger::fmt::Color::Rgb(140, 143, 145));
            writeln!(
                buf,
                "{} [{}] {}{} {}:{} - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                style.value(level),
                record.target(),
                target_style.value("]"),
                get_file_name(record.file()),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();
}
