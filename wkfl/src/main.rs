use std::{env, error::Error};

use clap::{Parser, Subcommand};

use git2::Repository;
use log::info;

mod git;
mod utils;

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
    End {
        name: Option<String>,
    },
    RepoDebug,
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

fn start_workflow(repo: Repository, name: &String, ticket: &Option<String>) -> anyhow::Result<()> {
    let user = utils::get_current_user().ok_or(anyhow::anyhow!("Unable to determine user"))?;
    let branch_name = match ticket {
        Some(ticket_key) => format!("{user}/{ticket_key}_{name}"),
        None => format!("{user}/{name}"),
    };

    if git::uses_worktrees(&repo) {
        info!("Creating worktree named '{name}' on branch '{branch_name}'");
        git::create_worktree(&repo, name, &branch_name)
    } else {
        info!("Creating branch '{branch_name}' and checking it out");
        git::switch_branch(&repo, &branch_name, true)
    }?;

    Ok(())
}

fn end_workflow(repo: Repository, name: &Option<String>) -> anyhow::Result<()> {
    if repo.is_worktree() {
        anyhow::bail!("For worktree based repos call stop from base of repo with name of worktree");
    } else if repo.is_bare() {
        let workspace_name = name.clone().ok_or(anyhow::anyhow!(
            "Must specify a name in worktree based repo"
        ))?;
        git::remove_worktree(&repo, &workspace_name)?;
    } else {
        match name {
            Some(branch_name) => git::remove_branch(&repo, branch_name)?,
            None => git::remove_current_branch(&repo)?,
        }
    }
    Ok(())
}

fn print_repo_debug_info(repo: Repository) -> anyhow::Result<()> {
    info!("worktree: {}", repo.is_worktree());
    info!("bare: {}", repo.is_bare());
    info!("state: {:?}", repo.state());
    info!("path: {:?}", repo.path());
    info!("workdir: {:?}", repo.workdir());
    info!("has_changes: {}", git::has_changes(&repo)?);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    let repo = git::get_repository()?;
    match &cli.command {
        Commands::Start { name, ticket } => start_workflow(repo, name, ticket)?,
        Commands::End { name } => end_workflow(repo, name)?,
        Commands::RepoDebug => {
            print_repo_debug_info(repo)?;
        }
    };

    Ok(())
}
