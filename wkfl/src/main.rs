use std::env;

use clap::{Parser, Subcommand};
use env_logger;
use log::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start {
        name: String,
        ticket: Option<String>,
    },
    End,
}

fn setup_logging(verbose: bool) {
    let mut log_builder = env_logger::builder();
    if verbose {
        log_builder.filter(None, log::LevelFilter::Debug);
    } else {
        // Only set default of info if not configured via env already
        if env::var("RUST_LOG").is_err() {
            log_builder.filter(None, log::LevelFilter::Info);
        }
        log_builder.format_timestamp(None);
    }
    log_builder.init();
}

fn main() {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    match &cli.command {
        Commands::Start { name, ticket } => {
            info!("'start' was used, name is: {name:?} and ticket is: {ticket:?}")
        }
        Commands::End => {
            info!("'end' was used")
        }
    }
}
