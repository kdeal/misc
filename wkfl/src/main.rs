use std::{env, error::Error, io, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::{generate, Shell};
use config::{ChatProvider, WebChatProvider};
use llm::ModelType;
use notes::DailyNoteSpecifier;

mod actions;
mod config;
mod git;
mod github;
mod jira;
mod llm;
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
    /// Enable verbose (debug) logging output.
    #[arg(short, long)]
    verbose: bool,
    /// Write generated shell integration commands to this file.
    #[arg(long, value_hint = ValueHint::FilePath)]
    shell_actions_file: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start working on a new feature.
    Start,
    /// End working on a feature.
    End,
    /// Print diagnostic information about the current Git repository.
    RepoDebug,
    /// List repositories in the configured repositories directory.
    Repos {
        /// Show absolute paths instead of relative names.
        #[arg(short, long)]
        full_path: bool,
    },
    /// Select a repository and switch to it.
    Repo,
    /// Print the currently resolved wkfl configuration.
    Config,
    /// Clone a repository into the repositories directory.
    Clone,
    /// List all local branches and delete those whose pull request has merged.
    PruneBranches,
    /// Run test commands defined in the repository configuration.
    Test {
        /// List configured commands without executing them.
        #[arg(long)]
        list: bool,
    },
    /// Run formatting commands defined in the repository configuration.
    Fmt {
        /// List configured commands without executing them.
        #[arg(long)]
        list: bool,
    },
    /// Run build commands defined in the repository configuration.
    Build {
        /// List configured commands without executing them.
        #[arg(long)]
        list: bool,
    },
    /// Prompt for a yes/no confirmation and exit non-zero on no.
    Confirm {
        /// Override the confirmation prompt text.
        #[arg(value_hint = ValueHint::Other)]
        prompt: Option<String>,
        /// Treat an empty response as an affirmative answer.
        #[arg(short = 't', long)]
        default_true: bool,
    },
    /// Prompt to select a value from newline-delimited stdin input.
    Select {
        /// Override the selection prompt text.
        #[arg(value_hint = ValueHint::Other)]
        prompt: Option<String>,
    },
    /// Open or create notes in the configured notes directory.
    Notes {
        #[command(subcommand)]
        command: NotesCommands,
    },
    /// Interact with configured large language model providers.
    Llm {
        #[command(subcommand)]
        command: LlmCommands,
    },
    /// Generate shell completion scripts for wkfl.
    Completion {
        /// Shell to generate completions for (defaults to current shell).
        language: Option<Shell>,
    },
    /// Send a grounded chat request using a web chat provider.
    WebChat {
        /// Prompt to send (prompts interactively if omitted).
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        /// Override the configured web chat provider.
        #[arg(short = 'p', long, value_enum)]
        model_provider: Option<WebChatProvider>,
        /// Model family to use for the chat request.
        #[arg(short, long, value_enum, default_value_t)]
        model_type: ModelType,
    },
    /// Send a chat request using a configured CLI chat provider.
    Chat {
        /// Prompt to send (prompts interactively if omitted).
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        /// Override the configured chat provider.
        #[arg(short = 'p', long, value_enum)]
        model_provider: Option<ChatProvider>,
        /// Model family to use for the chat request.
        #[arg(short, long, value_enum, default_value_t)]
        model_type: ModelType,
    },
    /// Query GitHub for information about the current repository.
    Github {
        #[command(subcommand)]
        command: GithubCommands,
    },
    /// Manage the shared todo list stored with your notes.
    Todo {
        #[command(subcommand)]
        command: TodoCommands,
    },
    /// Interact with Jira using configured credentials.
    Jira {
        #[command(subcommand)]
        command: JiraCommands,
    },
}

#[derive(Subcommand, Debug)]
enum NotesCommands {
    /// Open yesterday's daily note in your editor.
    Yesterday,
    /// Open today's daily note in your editor.
    Today,
    /// Open tomorrow's daily note in your editor.
    Tomorrow,
    /// Open or create a topic note by name.
    Topic {
        /// Name of the topic note to open (prompts if omitted).
        #[arg(value_hint = ValueHint::Other)]
        name: Option<String>,
    },
    /// Open or create a note for a specific person.
    Person {
        /// Person whose note to open (prompts if omitted).
        #[arg(value_hint = ValueHint::Other)]
        who: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum GithubCommands {
    /// List pull requests associated with a commit.
    #[command(name = "get_pr")]
    GetPr {
        /// Commit SHA to inspect (defaults to the current HEAD).
        #[arg(value_hint = ValueHint::Other)]
        commit_sha: Option<String>,
    },
    /// Print comments for a pull request.
    #[command(name = "get_pr_comments")]
    GetPrComments {
        /// Pull request number to inspect (defaults to the current commit's PR).
        #[arg(value_hint = ValueHint::Other)]
        pr_number: Option<u64>,
        /// Exclude general timeline comments from the output.
        #[arg(long)]
        filter_timeline: bool,
        /// Include bot-authored comments that are filtered by default.
        #[arg(long)]
        no_filter_bots: bool,
        /// Exclude diff review comments from the output.
        #[arg(long)]
        filter_diff: bool,
    },
}

#[derive(Subcommand, Debug)]
enum TodoCommands {
    /// Display todo items with optional filtering.
    List {
        /// Show only pending (unchecked) items.
        #[arg(short, long)]
        pending: bool,
        /// Show only completed (checked) items.
        #[arg(short, long)]
        completed: bool,
        /// Show only the count of items.
        #[arg(short = 'n', long)]
        count: bool,
    },
    /// Add a new item to the todo list.
    Add {
        /// Text of the new todo item.
        #[arg(value_hint = ValueHint::Other)]
        description: String,
        /// Add item at the top of the list.
        #[arg(short, long, conflicts_with = "after")]
        top: bool,
        /// Add item after the specified 1-based index.
        #[arg(short, long, conflicts_with = "top")]
        after: Option<usize>,
        /// Nest item under the previous item (increases indentation).
        #[arg(short, long)]
        nest: bool,
    },
    /// Remove an item from the todo list by index.
    Remove {
        /// 1-based index of the item to remove.
        #[arg(value_hint = ValueHint::Other)]
        index: Option<usize>,
    },
    /// Mark a todo item as completed.
    Check {
        /// 1-based index of the item to mark as completed.
        #[arg(value_hint = ValueHint::Other)]
        index: Option<usize>,
    },
    /// Mark a todo item as pending.
    Uncheck {
        /// 1-based index of the item to mark as pending.
        #[arg(value_hint = ValueHint::Other)]
        index: Option<usize>,
    },
    /// Open the todo.md file in the default editor.
    Edit,
}

#[derive(Subcommand, Debug)]
enum LlmCommands {
    /// Send a chat request using the Anthropic API.
    Anthropic {
        /// Prompt to send (prompts interactively if omitted).
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        /// Stream the response instead of waiting for completion.
        #[arg(short, long)]
        stream: bool,
    },
    /// Send a chat request using the Perplexity API.
    Perplexity {
        /// Prompt to send (prompts interactively if omitted).
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        /// Stream the response instead of waiting for completion.
        #[arg(short, long)]
        stream: bool,
    },
    /// Send a chat request using Google Vertex AI.
    VertexAi {
        /// Prompt to send (prompts interactively if omitted).
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
        /// Enable Google Search grounding for the request.
        #[arg(short, long)]
        enable_search: bool,
        /// Stream the response instead of waiting for completion.
        #[arg(short, long)]
        stream: bool,
    },
}

#[derive(Subcommand, Debug)]
enum JiraCommands {
    /// Fetch details for a Jira issue by key.
    Get {
        /// Issue key to fetch (for example, PROJ-123).
        #[arg(value_hint = ValueHint::Other)]
        issue_key: String,
    },
    /// Search Jira issues using a JQL query.
    Search {
        /// JQL query string to execute.
        #[arg(value_hint = ValueHint::Other)]
        jql: String,
        /// Maximum number of results to return.
        #[arg(short, long)]
        max_results: Option<u32>,
    },
    /// Search Jira issues using a saved filter.
    Filter {
        /// Filter ID to run (uses configured default when omitted).
        #[arg(long)]
        filter_id: Option<String>,
        /// Maximum number of results to return.
        #[arg(short, long)]
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
        Commands::Test { list } => actions::run_test_commands(&mut context, list)?,
        Commands::Fmt { list } => actions::run_fmt_commands(&mut context, list)?,
        Commands::Build { list } => actions::run_build_commands(&mut context, list)?,
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
            JiraCommands::Filter {
                filter_id,
                max_results,
            } => actions::search_jira_issues_by_filter(filter_id, max_results, &context.config)?,
        },
    };

    if let Some(shell_actions_file) = cli.shell_actions_file {
        shell_actions::write_shell_commands(&context.shell_actions, shell_actions_file)?;
    }

    Ok(())
}
