use std::{env, error::Error, io, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::{generate, Shell};
use notes::DailyNoteSpecifier;

mod actions;
mod config;
mod git;
mod llm;
mod notes;
mod prompts;
mod repositories;
mod shell_actions;
mod utils;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,
    #[arg(long, value_hint = ValueHint::FilePath)]
    shell_actions_file: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start,
    End,
    RepoDebug,
    Repos,
    Repo,
    Config,
    Clone,
    Confirm {
        #[arg(value_hint = ValueHint::Other)]
        prompt: Option<String>,
        #[arg(short = 't', long)]
        default_true: bool,
    },
    Select {
        #[arg(value_hint = ValueHint::Other)]
        prompt: Option<String>,
    },
    Notes {
        #[command(subcommand)]
        command: NotesCommands,
    },
    Llm {
        #[command(subcommand)]
        command: LlmCommands,
    },
    Completion {
        language: Option<Shell>,
    },
}

#[derive(Subcommand, Debug)]
enum NotesCommands {
    Yesterday,
    Today,
    Tomorrow,
    Topic {
        #[arg(value_hint = ValueHint::Other)]
        name: Option<String>,
    },
    Person {
        #[arg(value_hint = ValueHint::Other)]
        who: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum LlmCommands {
    Anthropic {
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
    },
    Perplexity {
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
    },
    VertexAi {
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        #[arg(short, long)]
        enable_search: bool,
    },
}

pub struct Context {
    config: config::Config,
    shell_actions: Vec<shell_actions::ShellAction>,
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

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    let mut context = Context {
        config: config::get_config()?,
        shell_actions: vec![],
    };
    match cli.command {
        Commands::Start => actions::start_workflow(&mut context)?,
        Commands::End => actions::end_workflow()?,
        Commands::RepoDebug => actions::print_repo_debug_info()?,
        Commands::Repos => actions::list_repositories(context.config)?,
        Commands::Repo => actions::switch_repo(&mut context)?,
        Commands::Clone => actions::clone_repo(&mut context)?,
        Commands::Config => actions::print_config(context.config),
        Commands::Confirm {
            prompt: user_prompt,
            default_true: default,
        } => {
            let prompt = user_prompt.unwrap_or("Confirm?".to_string());
            actions::confirm(&prompt, default)?
        }
        Commands::Select {
            prompt: user_prompt,
        } => {
            let prompt = user_prompt.unwrap_or("?".to_string());
            actions::select(&prompt)?
        }
        Commands::Notes {
            command: notes_command,
        } => match notes_command {
            NotesCommands::Yesterday => {
                actions::open_daily_note(DailyNoteSpecifier::Yesterday, &mut context)?
            }
            NotesCommands::Today => {
                actions::open_daily_note(DailyNoteSpecifier::Today, &mut context)?
            }
            NotesCommands::Tomorrow => {
                actions::open_daily_note(DailyNoteSpecifier::Tomorrow, &mut context)?
            }
            NotesCommands::Topic { name } => actions::open_topic_note(name, &mut context)?,
            NotesCommands::Person { who } => actions::open_person_note(who, &mut context)?,
        },
        Commands::Llm {
            command: llm_command,
        } => match llm_command {
            LlmCommands::Perplexity { query } => {
                actions::run_perplexity_query(query, context.config)?
            }
            LlmCommands::Anthropic { query } => {
                actions::run_anthropic_query(query, context.config)?
            }
            LlmCommands::VertexAi { query, enable_search } => actions::run_vertex_ai_query(query, enable_search, context.config)?,
        },
        Commands::Completion { language } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            let shell = language.unwrap_or(Shell::from_env().unwrap_or(Shell::Bash));
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
        }
    };

    if let Some(shell_actions_file) = cli.shell_actions_file {
        shell_actions::write_shell_commands(&context.shell_actions, shell_actions_file)?;
    }

    Ok(())
}
