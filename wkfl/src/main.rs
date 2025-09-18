use std::{env, error::Error, io, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::{generate, Shell};
use config::{ChatProvider, WebChatProvider};
use llm::ModelType;
use notes::DailyNoteSpecifier;

mod actions;
mod clients;
mod config;
mod git;
mod llm;
mod mcp;
mod notes;
mod prompts;
mod repositories;
mod shell_actions;
mod todo;
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
    Repos {
        #[arg(short, long)]
        full_path: bool,
    },
    Repo,
    Config,
    Clone,
    /// List all local branches and delete those whose pull request has been merged
    PruneBranches,
    /// Run test commands defined in repo config
    Test,
    /// Run fmt commands defined in repo config
    Fmt,
    /// Run build commands defined in repo config
    Build,
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
    WebChat {
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        #[arg(short = 'p', long, value_enum)]
        model_provider: Option<WebChatProvider>,
        #[arg(short, long, value_enum, default_value_t)]
        model_type: ModelType,
    },
    Chat {
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        #[arg(short = 'p', long, value_enum)]
        model_provider: Option<ChatProvider>,
        #[arg(short, long, value_enum, default_value_t)]
        model_type: ModelType,
    },
    /// Start MCP server
    Mcp,
    Github {
        #[command(subcommand)]
        command: GithubCommands,
    },
    Todo {
        #[command(subcommand)]
        command: TodoCommands,
    },
    Jira {
        #[command(subcommand)]
        command: JiraCommands,
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
enum GithubCommands {
    #[command(name = "get_pr")]
    GetPr {
        #[arg(value_hint = ValueHint::Other)]
        commit_sha: Option<String>,
    },
    #[command(name = "get_pr_comments")]
    GetPrComments {
        #[arg(value_hint = ValueHint::Other)]
        pr_number: Option<u64>,
        /// Filter out timeline comments (general PR comments)
        #[arg(long)]
        filter_timeline: bool,
        /// Filter out bot comments (default: true)
        #[arg(long)]
        no_filter_bots: bool,
        /// Filter out diff/review comments
        #[arg(long)]
        filter_diff: bool,
    },
}

#[derive(Subcommand, Debug)]
enum TodoCommands {
    List {
        #[arg(short, long, help = "Show only pending (unchecked) items")]
        pending: bool,
        #[arg(short, long, help = "Show only completed (checked) items")]
        completed: bool,
        #[arg(short = 'n', long, help = "Show only the count of items")]
        count: bool,
    },
    Add {
        #[arg(value_hint = ValueHint::Other)]
        description: String,
        #[arg(
            short,
            long,
            conflicts_with = "after",
            help = "Add item at the top of the list"
        )]
        top: bool,
        #[arg(
            short,
            long,
            conflicts_with = "top",
            help = "Add item after the specified 1-based index"
        )]
        after: Option<usize>,
        #[arg(
            short,
            long,
            help = "Nest item under the previous item (increases indentation)"
        )]
        nest: bool,
    },
    Remove {
        #[arg(value_hint = ValueHint::Other, help = "1-based index of the item to remove")]
        index: Option<usize>,
    },
    Check {
        #[arg(value_hint = ValueHint::Other, help = "1-based index of the item to mark as completed")]
        index: Option<usize>,
    },
    Uncheck {
        #[arg(value_hint = ValueHint::Other, help = "1-based index of the item to mark as pending")]
        index: Option<usize>,
    },
    #[command(about = "Open the todo.md file in the default editor")]
    Edit,
}

#[derive(Subcommand, Debug)]
enum LlmCommands {
    Anthropic {
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        #[arg(short, long)]
        stream: bool,
    },
    Perplexity {
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        #[arg(short, long)]
        stream: bool,
    },
    VertexAi {
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        #[arg(short, long)]
        enable_search: bool,
        #[arg(short, long)]
        stream: bool,
    },
}

#[derive(Subcommand, Debug)]
enum JiraCommands {
    Get {
        #[arg(value_hint = ValueHint::Other)]
        issue_key: String,
    },
    Search {
        #[arg(value_hint = ValueHint::Other)]
        jql: String,
        #[arg(short, long, help = "Maximum number of results to return")]
        max_results: Option<u32>,
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
        Commands::Repos { full_path } => actions::list_repositories(context.config, full_path)?,
        Commands::Repo => actions::switch_repo(&mut context)?,
        Commands::Clone => actions::clone_repo(&mut context)?,
        Commands::PruneBranches => actions::prune_merged_branches(&context.config)?,
        Commands::Test => actions::run_test_commands(&mut context)?,
        Commands::Fmt => actions::run_fmt_commands(&mut context)?,
        Commands::Build => actions::run_build_commands(&mut context)?,
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
            LlmCommands::Perplexity { query, stream } => {
                if stream {
                    actions::stream_perplexity_query(query, context.config)?
                } else {
                    actions::run_perplexity_query(query, context.config)?
                }
            }
            LlmCommands::Anthropic { query, stream } => {
                if stream {
                    actions::stream_anthropic_query(query, context.config)?
                } else {
                    actions::run_anthropic_query(query, context.config)?
                }
            }
            LlmCommands::VertexAi {
                query,
                enable_search,
                stream,
            } => {
                if stream {
                    actions::stream_vertex_ai_query(query, enable_search, context.config)?
                } else {
                    actions::run_vertex_ai_query(query, enable_search, context.config)?
                }
            }
        },
        Commands::Completion { language } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            let shell = language.unwrap_or(Shell::from_env().unwrap_or(Shell::Bash));
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
        }
        Commands::WebChat {
            query,
            model_type,
            model_provider,
        } => actions::run_web_chat(query, model_type, model_provider, context.config)?,
        Commands::Chat {
            query,
            model_type,
            model_provider,
        } => actions::run_chat(query, model_type, model_provider, context.config)?,
        Commands::Mcp => {
            let server = mcp::McpServer::new();
            server.run()?
        }
        Commands::Github {
            command: github_command,
        } => match github_command {
            GithubCommands::GetPr { commit_sha } => {
                actions::get_pull_request_for_commit(commit_sha, &context.config)?
            }
            GithubCommands::GetPrComments {
                pr_number,
                filter_timeline,
                no_filter_bots,
                filter_diff,
            } => actions::get_pr_comments(
                pr_number,
                filter_timeline,
                !no_filter_bots,
                filter_diff,
                &context.config,
            )?,
        },
        Commands::Todo {
            command: todo_command,
        } => match todo_command {
            TodoCommands::List {
                pending,
                completed,
                count,
            } => todo::list_todos(&context.config, pending, completed, count)?,
            TodoCommands::Add {
                description,
                top,
                after,
                nest,
            } => todo::add_todo(&context.config, description.clone(), top, after, nest)?,
            TodoCommands::Remove { index } => todo::remove_todo(&context.config, index)?,
            TodoCommands::Check { index } => todo::check_todo(&context.config, index)?,
            TodoCommands::Uncheck { index } => todo::uncheck_todo(&context.config, index)?,
            TodoCommands::Edit => todo::edit_todo(&mut context)?,
        },
        Commands::Jira {
            command: jira_command,
        } => match jira_command {
            JiraCommands::Get { issue_key } => {
                actions::get_jira_issue(&issue_key, &context.config)?
            }
            JiraCommands::Search { jql, max_results } => {
                actions::search_jira_issues(&jql, max_results, &context.config)?
            }
        },
    };

    if let Some(shell_actions_file) = cli.shell_actions_file {
        shell_actions::write_shell_commands(&context.shell_actions, shell_actions_file)?;
    }

    Ok(())
}
