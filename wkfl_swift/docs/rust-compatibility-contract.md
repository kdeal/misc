# Rust Compatibility Contract

## Status and scope

This document is the normative migration contract for the Swift implementation of `wkfl`. It
describes the user-visible behavior of the Rust implementation at repository revision `3ebe4fe3`
and records every intentional Swift difference known at the start of the port.

Later porting work must preserve the behavior marked **Preserve**. Behavior marked **Fix** is not a
compatibility requirement and must use the replacement contract stated here. A new difference must
update this document before or with the implementation that introduces it.

The reference implementation is primarily:

- `wkfl/src/main.rs` for commands, options, dispatch, and completion.
- `wkfl/src/actions.rs` for command output.
- `wkfl/src/config.rs` and `wkfl/docs/configuration.md` for configuration.
- `wkfl/src/shell_actions.rs` and `wkfl/shell_wrappers/wkfl.fish` for shell integration.
- `wkfl/src/github.rs`, `wkfl/src/jira.rs`, and `wkfl/src/llm/*` for service contracts.

## Compatibility rules

### Process behavior

- **Preserve:** the executable is named `wkfl`; it requires a subcommand.
- **Preserve:** `-v`/`--verbose` enables debug logging. Logs and interactive prompts go to stderr.
  Command results, generated completion scripts, and JSON go to stdout. Shell actions go only to
  the requested action file. This separation keeps stdout pipeable.
- **Preserve:** successful commands and documented empty-result cases exit `0`, CLI usage errors
  exit `2`, and runtime failures exit nonzero.
- **Fix:** runtime failures must be ordinary, actionable errors. Swift must not reproduce Rust
  panics, unchecked indexing, or `expect` failures. A negative `tui confirm` remains a quiet exit `1`.
- **Preserve:** JSON is pretty printed and ends with a newline. `wkfl`-defined keys use snake_case;
  provider-shaped Jira and ADF keys retain their documented casing. Prompts and logs may still
  appear on stderr in JSON mode. Runtime errors are not JSON envelopes unless a command explicitly
  defines one below.
- **Preserve:** ANSI color, OSC 8 links, and OSC 9 notifications are part of human terminal output
  where noted. They must never appear in JSON.
- **Fix:** terminal state restoration is unconditional on success, error, cancellation, or thrown
  errors. User-controlled terminal text must escape or remove control sequences.
- **Preserve:** commands that use configuration load it before dispatch; missing configuration is
  equivalent to an empty file.
- **Fix:** `completion`, `tui confirm`, and `tui select` dispatch without resolving the home
  directory or loading/parsing global configuration. Invalid or unreadable config must not prevent
  these commands from running.

### Naming

- **Preserve:** normal multiword commands and options are kebab-case.
- **Fix:** all Swift GitHub subcommands use kebab-case: `prs-to-review`, `get-pr`,
  `get-pr-for-commit`, `get-pr-comments`, `mark-thread-read`, and `mark-thread-done`. The Rust
  snake_case spellings are not retained as aliases.
- **Fix:** Rust's top-level `repo-debug`, `repos`, `clone`, `test`, `fmt`, and `build` commands move
  under the `repo` command without top-level aliases.
- **Fix:** Rust's `prune-branches` command is removed without a Swift replacement or alias.
- **Fix:** Rust's top-level `confirm` and `select` commands move under `tui` without top-level
  aliases. Bare `tui` is a missing-subcommand usage error.
- **Preserve:** commands have no aliases unless this contract explicitly adds one.
- **Preserve:** global options occur before the subcommand: `-v`/`--verbose` and
  `--shell-actions-file PATH`.
- **Preserve:** `-h`/`--help`, `-V`/`--version`, and `help [COMMAND ...]` are available. Help and
  version go to stdout and exit `0`; invalid usage goes to stderr and exits `2`. The initial Swift
  port reports compatibility version `0.1.4`, and version is available below subcommands.

## CLI inventory

Unless stated otherwise, boolean flags default to false, optional values default to absent, human
output goes to stdout, prompts go to stderr, and runtime errors go to stderr with a nonzero exit.

### Repository commands

The Swift form is `repo [SUBCOMMAND]`. With no subcommand, `repo` runs the Rust `repo` behavior:
interactively select a discovered repository and queue a `cd` shell action. The nested command names
are `cd`, `debug`, `list`, `clone`, `test`, `fmt`, and `build`. `repo cd` is an explicit subcommand
with exactly the same behavior as bare `repo`.

| Invocation | Inputs and defaults | Output and side effects | Decision |
| --- | --- | --- | --- |
| `repo` | Interactive selector over discovered repositories. | No stdout; queues a `cd` shell action for the selected full path. | **Preserve** selection semantics. **Fix** empty inventory to a normal error. |
| `repo cd` | Same inputs and behavior as bare `repo`. | Same selector, output channels, and `cd` shell action as bare `repo`. | **Fix:** add an explicit spelling for the default behavior. |
| `repo debug` | No arguments. Current Git repository required. | Repository worktree, bare/state/path/workdir/change/worktree diagnostics are logs on stderr; no stdout. | **Fix** command location; preserve fields and channel. Swift formatting need not imitate Rust `Debug`. |
| `repo list [-f\|--full-path] [--json]` | Search `repositories_directory`; relative paths by default. | Text is one path per line. JSON is `{"base_directory": string, "repos": [string]}`. | **Fix** command name/location and ordering to lexical relative-path order; preserve schema and safely handle symlinks/unreadable entries. |
| `repo clone` | Prompts `Clone Url:`. | Clones beneath `repositories_directory`; queues `cd` on success. Prefer `jj git clone` when `jj` is available, otherwise `git clone`. | **Fix** command location, path traversal, and retry with Git when `jj` is unavailable or reports that the target is not supported; preserve preference and destination layout. Do not retry authentication/network failures blindly. |
| `repo test [--list]` | Repository `test_commands`. | `--list` prints commands. Otherwise prints each command in yellow and executes sequentially with `sh -c`; first failure aborts. | **Fix** command location; preserve behavior. |
| `repo fmt [--list]` | Repository `fmt_commands`. | Same protocol as `repo test`. | **Fix** command location; preserve behavior. |
| `repo build [--list]` | Repository `build_commands`. | Same protocol as `repo test`. | **Fix** command location; preserve behavior. |

When a workflow list is empty, print `No <test|fmt|build> commands configured in repository config`
and exit `0`. A successful run prints a green `✓ All commands completed successfully` and an OSC
9 notification. A failed command prints a red `✖ Command failed`, sends an OSC 9 notification, and
returns a nonzero error naming the command.

Configured workflow subprocesses inherit the CLI's stdin, stdout, and stderr. Their stdout remains
stdout and their stderr remains stderr; only wkfl's command announcement and final status are
wkfl-generated stdout.

The Rust `prune-branches` command has no Swift equivalent. Both `wkfl prune-branches` and
`wkfl repo prune-branches` are unknown-command usage errors.

### Configuration command

| Invocation | Inputs and defaults | Output | Decision |
| --- | --- | --- | --- |
| `config` | No arguments. | Rust logs a `Debug` dump on stderr. | **Fix:** print the deterministic effective TOML defined below to stdout. Never emit resolved credentials. |

### TUI commands

The Swift form is `tui <SUBCOMMAND>`. It groups the interactive utility commands and requires either
`confirm` or `select`.

| Invocation | Inputs and defaults | Output | Decision |
| --- | --- | --- | --- |
| `tui confirm [PROMPT] [-t\|--default-true]` | Prompt defaults to `Confirm?`; answer defaults false unless `-t`; does not load config. | UI on stderr; yes exits `0`, no exits `1` without an error message. | **Fix** command location and config independence; preserve keys and exit behavior. |
| `tui select [PROMPT]` | Prompt defaults to `?`; options are nonempty lines read from stdin to EOF; does not load config. | UI and selected-line echo on stderr; raw selected value plus newline on stdout. | **Fix** command location, config independence, empty input, and unsafe terminal text; preserve stream behavior. |

For `tui confirm`, Enter accepts the current state; `h`, `t`, `y`, and Left select true; `l`, `f`,
`n`, and Right select false; Ctrl-C is a runtime cancellation error. Prompt editing preserves the
Rust insert/normal-mode user model. Selectors use Up/Down, normal-mode `j`/`k`, and insert-mode
Ctrl-N/Ctrl-P, smart-case fuzzy matching, and at most ten visible options. Exact cursor rendering is
not compatibility-sensitive.

### Completion command

| Invocation | Inputs and defaults | Output | Decision |
| --- | --- | --- | --- |
| `completion [LANGUAGE]` | `bash`, `elvish`, `fish`, `powershell`, or `zsh`; infer from `SHELL`; default Bash when inference fails; does not load config. | Completion script on stdout. | **Preserve** all five values and inference, even if a generator must be implemented outside Swift Argument Parser; **Fix** config independence. |

### Notes

| Invocation | Inputs and defaults | Result | Decision |
| --- | --- | --- | --- |
| `notes yesterday` | No arguments. | Open/create yesterday's daily note through an `edit_file` shell action. | **Preserve** layout; **Fix** date zone. |
| `notes today` | No arguments. | Open/create today's daily note. | Same. |
| `notes tomorrow` | No arguments. | Open/create tomorrow's daily note. | Same. |
| `notes topic [NAME]` | Prompt `Topic Name:` when omitted. | `topics/<normalized>.md`, initial heading title-cased. | **Preserve** safe-name normalization; **Fix** traversal. |
| `notes person [WHO]` | Prompt `Who:` when omitted. | `people/<normalized>.md`, initial heading preserves input. | **Preserve** safe-name normalization; **Fix** traversal. |

Daily files remain `daily/YYYY/<Sunday-based-week>/<short-weekday>_<short-month>_<day>.md` with the
template `# <long-weekday> <long-month> <day><ordinal>\n\n## `. Existing files are never
overwritten. **Fix:** Swift uses the user's local calendar rather than the Rust implementation's
effective UTC date. Topic/person names still lowercase and replace spaces and hyphens with `_`, but
Swift rejects path separators, `..`, NUL, newlines, and control characters.

### Todo

The file remains `<notes_directory>/todo.md` with this canonical persisted format:

```markdown
# Todo List
- [ ] pending
    - [x] nested complete
```

| Invocation | Inputs and defaults | Output | Decision |
| --- | --- | --- | --- |
| `todo list [-p\|--pending] [-c\|--completed] [-n\|--count]` | Neither status flag means all; both also mean all. | Count only, or indexed checkbox lines headed by item count. Empty lists use the Rust status-specific messages. | **Preserve**. |
| `todo add DESCRIPTION [-t\|--top] [-a\|--after INDEX] [-n\|--nest]` | End by default; top and after conflict; index is one-based; nest increases indentation from the preceding item. | `Added todo item: <description>`. | **Preserve**; **Fix** empty/newline/control input rejection. |
| `todo remove [INDEX]` | Interactive selection when index is absent. | `Removed todo item: <description>`. | **Preserve**. |
| `todo check [INDEX]` | Interactive pending-item selection when absent. | `Marked as completed: <description>`. | **Preserve**. |
| `todo uncheck [INDEX]` | Interactive completed-item selection when absent. | `Marked as pending: <description>`. | **Preserve**. |
| `todo edit` | No arguments. | Creates the canonical file if absent and queues `edit_file`; no stdout. | **Preserve**. |

The parser trims the heading and requires it to equal `# Todo List`; whitespace-only lines and
checkbox records with lowercase `[x]` are accepted. Indentation is normalized to four spaces per
level. **Fix:** descriptions must contain at least one non-whitespace character and no newline or
control characters. Swift creates the notes directory, uses an exclusive unique same-directory
temporary file for atomic replacement, and does not recover an item identity by parsing its
rendered selector label.

### LLM commands

For every optional `QUERY`, use the argument when supplied, prompt `Query:` when stdin is a terminal,
or otherwise consume stdin in full, including trailing newlines.

| Invocation | Inputs and defaults | Output | Decision |
| --- | --- | --- | --- |
| `llm anthropic [QUERY] [-s\|--stream]` | Direct model `claude-sonnet-4-5-20250929`; 1024 max tokens. | Nonstream: print the first content block only when it is text, then newline; a non-text first block emits nothing, while absent content is a provider error. Stream: incremental text; thinking appears as `\n[Thinking] ...`; final newline. | **Preserve**, except stream error events must fail the command after printing the provider error to stderr. |
| `llm perplexity [QUERY] [-s\|--stream]` | Direct-command model `sonar`. | Nonstream citations then answer; stream answer then citations; final newline. | **Fix** citation numbering and delimiters as below. |
| `llm vertex-ai [QUERY] [-e\|--enable-search] [-s\|--stream]` | Search off by default; fixed location and model family. | Nonstream citations before answer; stream answer then citations. | **Preserve** order and search flag; **Fix** model IDs. |
| `chat [QUERY] [-p\|--model-provider PROVIDER] [-m\|--model-type TYPE]` | Providers `vertex-ai`, `anthropic`, `ollama`; types `small` (default), `large`, `thinking`. | Message content plus newline. | **Preserve** provider precedence and model family mapping. Missing provider is a normal config error. |
| `web-chat [QUERY] [-p\|--model-provider PROVIDER] [-m\|--model-type TYPE]` | Providers `vertex-ai`, `perplexity`; same model types/default. | Answer with citation markers, two newlines, OSC 8 source links, final newline. | **Preserve** shape; **Fix** one-based markers and Unicode handling. Vertex requests enable Search. |

Provider auto-selection remains:

1. Explicit CLI provider.
2. Configured `chat_provider` or `web_chat_provider`.
3. Chat: Anthropic, then Vertex AI, then Ollama, based on configured sections.
4. Web chat: Perplexity, then Vertex AI, based on configured sections.

### GitHub

The parent form is `github [--hostname HOSTNAME] <SUBCOMMAND>`. GitHub.com uses
`https://api.github.com` and `/graphql`; enterprise hosts use HTTPS with `/api/v3` and
`/api/graphql`. Tokens are selected by exact hostname from `[github_tokens]`.

| Invocation | Inputs and defaults | Output | Decision |
| --- | --- | --- | --- |
| `github prs-to-review [--json] [--include-teams]` | Current host unless `--hostname`; individual review requests by default, team requests included with flag. | Human PR blocks or no-results sentence; JSON array described below. | **Fix** Rust command spelling; preserve behavior. |
| `github get-pr [PR_NUMBER] [--repo OWNER/NAME] [--json]` | Missing number selects the first PR associated with current HEAD. `--repo` and `--hostname` must be supplied together. | Markdown details or PR detail JSON. | **Fix** Rust command spelling; preserve first-PR behavior and pairing rule. |
| `github get-pr-for-commit [COMMIT_SHA] [--repo OWNER/NAME] [--json]` | SHA defaults to HEAD; same repo/host pairing. | `PR #N (<status>): URL`, no-result sentence, or compact PR JSON array. | **Fix** Rust command spelling and closed-unmerged status to `closed`; preserve other behavior. |
| `github get-pr-comments [PR_NUMBER] [--repo OWNER/NAME] [--filter-timeline] [--no-filter-bots] [--filter-diff] [--json]` | PR defaults through HEAD. `--repo` and `--hostname` must be supplied together. Bots are filtered by default; the negative flag includes them. | Markdown comment sections or filtered comment JSON. | **Fix** Rust command spelling; preserve flag inversion and schemas. Sanitize terminal controls only. |
| `github notifications [--since TIMESTAMP] [--all] [--json]` | Unread by default; `since` passed as ISO 8601; current host unless overridden. | Notification blocks/no-results sentence or JSON array. | **Preserve**, but validate ISO 8601 locally. |
| `github mark-thread-read THREAD_ID` | Current host unless overridden. | `Marked GitHub notification thread <id> as read`. | **Fix** Rust command spelling; preserve behavior. |
| `github mark-thread-done THREAD_ID` | Current host unless overridden. | `Marked GitHub notification thread <id> as done`. | **Fix** Rust command spelling; preserve behavior. |

REST list pagination uses 100 items per page. Detail queries paginate all files, commits, reviews,
comments, threads, requests, thread comments, and status-check rollup contexts. **Fix:** the
commit-associated PR endpoint used by `get-pr-for-commit` and implicit PR resolution also paginates
rather than silently stopping at GitHub's default first page. For check rollups,
`total_count` remains GitHub's count of all rollup contexts while `check_runs` contains every
context whose type is `CheckRun` and `status_contexts` contains every `StatusContext`. Other context
types are intentionally excluded from both arrays. A deleted head repository uses a `null` label as
specified in the JSON contract rather than substituting the base repository.

`get-pr` Markdown preserves the expanded Rust output added in revision `3ebe4fe3`: Latest Status
lists legacy status contexts with descriptions and target URLs; Latest Checks lists check-run detail
URLs and rollup status contexts; Reviews render each review's author, state, submission time, and
nonempty body.

Bot matching preserves Rust behavior: GitHub type `Bot`, login beginning with case-sensitive
`service`, or login ending with case-sensitive `[bot]`.

### Jira

The parent form is `jira <SUBCOMMAND>`. It uses Jira REST API v3 and Basic authentication from
`[jira]` (`email` plus resolved `api_token`).

| Invocation | Inputs and defaults | Output | Decision |
| --- | --- | --- | --- |
| `jira get ISSUE_KEY [--json]` | Required unvalidated issue key. | Human issue/details/comments or issue JSON. | **Preserve**. |
| `jira search JQL [-m\|--max-results N] [--json]` | Omitted maximum means all results; API page size is at most 100. | Fixed-column table/no-results sentence or issue JSON array. | **Preserve** user semantics; **Fix** transport to support Jira's token pagination. |
| `jira filter [--filter-id ID] [-m\|--max-results N] [--json]` | ID fetches directly. Omitted ID lists favourite filters and prompts `Select a filter:`. There is no configured default. | Named table, filter/issues JSON, or documented no-favourites result. | **Fix** misleading help text; preserve actual favourite-selector behavior. |

No favourite filters remains a successful empty result. Human output is
`No favourite filters found. You can add filters to your favourites in Jira.`; JSON is
`{"error": "<same message>"}` and exits `0`. Duplicate filter display names must be represented by
opaque IDs and select the intended filter rather than the first matching label.

ADF-to-Markdown output preserves paragraphs, headings, blockquotes, code blocks, hard breaks, rules,
bullet/ordered/task/decision lists, panels, tables, marks, links, mentions, emoji, dates, statuses,
media, cards, expand markers, extensions, and unknown-node omission. Swift fixes file media URL
handling, table pipe/newline escaping, ordered-list `order`, decision state rendering, and unknown
attribute handling. These Markdown fixes do not alter the preserved ADF object in JSON output.

## Configuration contract

### Global configuration

The path remains exactly `~/.config/wkfl/config.toml`; `XDG_CONFIG_HOME` is not consulted. Only a
leading `~/` is expanded. Relative paths remain relative to the process working directory. Unknown
TOML keys are ignored for forward compatibility.

| Key | Type/default | Contract |
| --- | --- | --- |
| `repositories_directory` | String, `"~/repos/"` | Repository discovery base. |
| `notes_directory` | Optional string | Defaults to `<repositories_directory>/notes`. |
| `chat_provider` | `VertexAI`, `Anthropic`, or `Ollama` | Optional provider override; spelling and case are exact. |
| `web_chat_provider` | `VertexAI` or `Perplexity` | Optional provider override; spelling and case are exact. |
| `anthropic_api_key` | Optional secret string | Enables Anthropic auto-selection. |
| `perplexity_api_key` | Optional secret string | Enables Perplexity auto-selection. |
| `[vertex_ai]` | `api_key`, `project_id` | Both required when the table exists. Only `api_key` is secret-resolved. |
| `[ollama]` | `base_url`, `small`, `large`, `thinking` | URL defaults on omission/blank; `small` required when used; large falls back to small and thinking to large. |
| `[github_tokens]` | Host-to-secret map | Empty by default; exact case-sensitive host lookup. |
| `[jira]` | `instance_url`, `email`, `api_token` | All required when the table exists; only token is secret-resolved. |

Secret fields support these exact prefixes:

- `env::NAME`: return the environment value without trimming.
- `cmd::COMMAND`: run `sh -c`, require success and UTF-8 stdout, then trim surrounding whitespace.
- `val::VALUE`: return the literal suffix.
- No prefix: use the value literally.

**Fix:** the Rust documentation incorrectly says every config value supports secret references.
Swift resolves only credentials explicitly identified as secret above. Empty credentials count as
invalid, not configured.

`config` prints the effective configuration as valid TOML with a final newline. Top-level keys use
the table order shown above; entries in `[github_tokens]` are sorted by hostname. Effective directory
values are expanded absolute paths. Absent optional keys/tables are omitted. Present secret fields
and token-map values are the literal string `"<redacted>"`; secret references are never resolved for
display. Non-secret provider, project, URL, model, and identity values are printed unchanged. This
ordering and redaction string are part of the command contract.

### Repository configuration

The schema remains:

```toml
test_commands = ["..."]
fmt_commands = ["..."]
build_commands = ["..."]
```

**Fix:** lookup is the documented repository-based order, regardless of the process subdirectory:

1. `<git-common-directory>/info/wkfl.toml`
2. `<repository-root>/.wkfl.toml`

The Rust implementation accidentally reads `<current-working-directory>/.wkfl.toml` and derives a
physical `.git/info` path from repository layout rather than asking Git for its common directory.
This breaks subdirectory lookup and can break nonstandard or bare layouts. Swift does not copy that
behavior. A later nonempty command list replaces the earlier list. For initial compatibility, an
empty later list does not clear an earlier one; a future clear operation requires a schema change.

## Repository and Git contract

- **Preserve:** repository discovery recognizes either `.git` or `.jj`, including a `.git` file for
  linked worktrees, and does not descend into a recognized repository.
- **Preserve:** hidden entries are skipped and a missing base directory produces an empty list.
- **Fix:** discovered repositories are sorted by relative path. Symlinks resolve relative to their
  parent, cycles are detected, and failures are errors rather than panics. A path outside the
  configured base is never returned through a followed symlink.
- **Fix:** repository paths that cannot be represented as valid UTF-8 cause a clear discovery error;
  text, JSON, selectors, and action files never use replacement-character paths.
- **Preserve:** `origin` is the preferred remote.
- **Fix:** remote URL lookup tries Jujutsu metadata when the repository is Jujutsu-backed, then the
  Git `origin` URL through the repository library, then `git remote get-url origin`. Ordinary Git
  repositories must not require `jj`.
- **Fix:** default branch lookup first uses `refs/remotes/origin/HEAD`, then remote metadata, then an
  existing `origin/main`, then `origin/master`. If none exists, return an actionable error.
- **Preserve:** clone supports `git@host:owner/repo[.git]` and standard URLs and creates the remote
  path below the repository base.
- **Fix:** clone destinations must remain canonically inside the base. Reject absolute paths, `..`,
  missing owner/repository segments, control characters, and extra path segments where an
  `owner/repository` identity is required.

## Shell-action contract

The Rust line protocol (`cd,<path>` and `edit_file,<path>`) cannot represent commas or newlines and
the Fish wrapper uses unsafe `eval`. It is intentionally not preserved.

**Fix:** Swift and its wrappers use UTF-8 JSON Lines, one object per queued action, in insertion
order:

```json
{"action":"cd","path":"/absolute/path"}
{"action":"edit_file","path":"/absolute/path"}
```

- The action file is created or truncated after successful command execution, including when there
  are no actions.
- Paths must be absolute, valid UTF-8, and JSON escaped. An unrepresentable path is an error before
  writing any records.
- The only action names are `cd` and `edit_file`. Unknown actions are rejected by wrappers.
- Wrappers parse JSON, pass the path as a distinct argument without `eval`, preserve the CLI exit
  status, clean up the temporary file, and execute no actions when the CLI fails.
- `edit_file` requires `EDITOR`. Its value is parsed as shell-style words without expansion,
  substitution, redirection, or evaluation, preserving safe fixed arguments such as `code --wait`;
  the action path is appended as one argument. A missing/invalid editor or failed `cd`/editor action
  makes the wrapper return nonzero. When the CLI itself fails, its original status is returned.
- This protocol changes Rust wrapper compatibility intentionally; the Swift binary and wrappers
  must ship together.

## JSON contracts

Dates remain provider strings unless explicitly described. Nullability and omission follow each
schema below. Exact field names and scalar types are compatibility requirements.

### Repositories

```json
{"base_directory":"string","repos":["string"]}
```

`base_directory` is the resolved configured base in both modes. `repos` entries are relative unless
`--full-path` is set.

### GitHub

- PRs for commit: array of `{number: number, merged_at: string|null, html_url: string}`.
- PRs to review: array of `{repo, repo_url, number, title, author:{login,type}, state, is_draft,
  url, created_at, updated_at}`.
- Notifications: array of `{id, unread, reason, updated_at, last_read_at,
  subject:{title,url,latest_comment_url,type}, repository:{full_name,html_url}, url,
  subscription_url}`. `last_read_at`, `subject.url`, and `subject.latest_comment_url` are string or
  `null`; their keys are never omitted.
- PR comments: `{issue_comments, review_comments}`. Issue comments contain `body`,
  `user:{login,type}`, and `created_at`. Review comments add `path`, `original_line`,
  `original_start_line`, `diff_hunk`, `side`, `start_side`, and `is_resolved`. `original_line`,
  `original_start_line`, and `start_side` are nullable keys; `is_resolved` is boolean.
- PR details: `{pull_request, diff, files, issue_comments, review_comments, review_threads, reviews,
  commits, status, check_runs}`.
- `pull_request`: `{number,title,html_url,state,body,created_at,updated_at,merged_at,additions,
  deletions,changed_files,user:{login,type},base:{label,sha},head:{label,sha},
  requested_reviewers:[{login,type}],requested_teams:[{name,slug}]}`.
- `merged_at` and `head.label` are string or `null`; `body` and all other base/head fields are
  strings. `head.label` is `null` when GitHub reports that the head repository was deleted.
- Files are `{filename,status,additions,deletions}`; reviews are
  `{user:{login,type},state,submitted_at,body}` with nullable `submitted_at` and string `body`;
  commits are `{sha,commit:{message}}`.
- Review threads are `{id,is_resolved,diff_side,start_diff_side,comments:[ReviewComment]}`. They do
  not duplicate review-comment fields at thread level. `start_diff_side` is nullable. Internal
  pagination metadata is omitted.
- A status context is `{context,state,description,target_url}`. `description` and `target_url` are
  nullable keys; `state` is lowercase.
- Status is `null` or `{sha,state,total_count,contexts:[StatusContext]}`.
- Check runs are `null` or
  `{total_count,check_runs:[{name,status,conclusion,details_url}],status_contexts:[StatusContext]}`.
  Each `conclusion` and `details_url` is a nullable key. When the latest commit has no status-check
  rollup, the object is `{total_count:0,check_runs:[]}` and omits `status_contexts`.

`review_comments` intentionally duplicates comments also nested in `review_threads` for Rust JSON
compatibility. `prs-to-review` results preserve GraphQL state casing. PR-detail `pull_request.state`, file
statuses, commit statuses, and check statuses/conclusions are lowercase.

### Jira

An issue is:

```text
{
  id: string, key: string,
  fields: {
    summary: string, description: ADF|null,
    status: {id:string,name:string,statusCategory:{key:string,name:string}},
    assignee: {displayName:string}|null, reporter: {displayName:string}|null,
    created: string, updated: string, priority: {id:string,name:string}|null,
    issuetype: {id:string,name:string,subtask:boolean},
    project: {id:string,name:string,key:string},
    comment: {comments:[{
      id:string,body:ADF,author:{displayName:string},created:string,updated:string
    }]}
  }
}
```

`ADF` is the complete JSON object received for the description or comment body. It has root
`{"type":"doc","version":number,"content":[Node]}`. Nodes and marks retain Jira's discriminator
and field casing, including `codeBlock`, `bulletList`, `orderedList`, `listItem`, `tableRow`,
`tableCell`, `tableHeader`, `mediaGroup`, `mediaSingle`, `nestedExpand`, `hardBreak`, `inlineCard`,
`blockCard`, `embedCard`, `taskList`, `taskItem`, `decisionList`, `decisionItem`,
`bodiedExtension`, `inlineExtension`, `textColor`, `backgroundColor`, and `dataConsumer`. Swift
preserves the entire ADF subtree, including unknown node/mark types, provider nulls, and omitted
properties, rather than reproducing Rust's lossy typed reserialization.

`jira get --json` returns one issue. `jira search --json` returns an issue array. Filter JSON is:

```text
{
  filter: {
    id:string, name:string, description:string|null, jql:string, favourite:boolean,
    owner:{displayName:string}
  },
  issues:[Issue]
}
```

Jira account IDs and email addresses remain excluded from serialized users. Provider-shaped keys
such as `displayName` and `statusCategory` intentionally remain camelCase.

## Provider and model contract

| Provider/family | Rust ID | Swift contract | Decision |
| --- | --- | --- | --- |
| Anthropic small | `claude-haiku-4-5-20251001` | Same pinned ID. | **Preserve**. |
| Anthropic large/direct | `claude-sonnet-4-5-20250929` | Same pinned ID. | **Preserve**. |
| Anthropic thinking | Sonnet above, thinking budget 1024, max tokens 2048 | Same. | **Preserve**. |
| Perplexity small/direct | `sonar` | Same. | **Preserve**. |
| Perplexity large | `sonar-pro` | Same. | **Preserve**. |
| Perplexity thinking | `sonar-reasoning-pro` | Same. | **Preserve**. |
| Vertex AI small/direct | `gemini-2.5-flash-preview-04-17` | `gemini-2.5-flash`. | **Fix** expired preview ID. |
| Vertex AI large/thinking | `gemini-2.5-pro-preview-03-25` | `gemini-2.5-pro`; thinking selects Pro without extra thinking config. | **Fix** expired preview ID; preserve family behavior. |
| Ollama | Configured names | `small` required; `large -> small`; `thinking -> large -> small`. | **Preserve**. |

Vertex AI remains in `us-central1` using the publisher `google` endpoint and bearer credential.
Model IDs are implementation constants, not new config fields in the initial port. Changing a model
ID later requires updating this table.

### Perplexity citations and Unicode

The Rust citation extractor assumes zero-based provider references, mixes character and UTF-8 byte
offsets, accepts an index equal to the source count, and can underflow before the first period. These
are defects, not compatibility requirements.

Swift must:

- Interpret provider references `[1]` through `[N]` as one-based source numbers.
- Render human citation labels and superscripts as one-based numbers.
- Use Swift `String.Index`/grapheme-safe ranges consistently; never slice Unicode with byte offsets.
- Handle references in the first sentence and text with no period or newline.
- Treat `.`, `!`, `?`, and line boundaries as sentence ends for support ranges.
- Reject or ignore `0`, values greater than the source count, malformed references, and counts that
  cannot be represented, without crashing.
- Preserve source order and remove only recognized inline citation tokens.
- In raw nonstream output, place each `[N] = URL` on its own line, then one blank line, then the
  answer. Streaming writes the answer, one blank line, then citation lines, and a final newline.
- Decode ordinary nonstream and stream response shapes without requiring both `message` and `delta`
  in every choice.

## Explicit discrepancy decisions

This index is intended for later issues to reference directly.

| ID | Rust discrepancy | Swift decision |
| --- | --- | --- |
| C01 | Repository `.wkfl.toml` is read from process CWD. | **Fix:** read repository root. |
| C02 | Git common-directory lookup is inferred from physical layout. | **Fix:** ask Git for its common directory. |
| C03 | Empty later command arrays cannot clear earlier arrays. | **Preserve initially**; require a versioned schema to add clearing. |
| C04 | Documentation claims secret prefixes work for every value. | **Fix:** only declared credential fields resolve secrets. |
| C05 | Empty credential strings count as configured. | **Fix:** reject them. |
| C06 | `config` logs unstable debug output and may reveal secrets. | **Fix:** redacted deterministic TOML on stdout. |
| C07 | Rust loads global config for `completion`, `confirm`, and `select`. | **Fix:** dispatch `completion`, `tui confirm`, and `tui select` without config or home-directory resolution. |
| R01 | Git remote lookup requires `jj` in Git repositories. | **Fix:** Jujutsu, library, then Git fallback. |
| R02 | Default branch requires `origin/HEAD`. | **Fix:** documented metadata/main/master fallback. |
| R03 | Repository order is filesystem-dependent. | **Fix:** lexical relative-path order. |
| R04 | Symlink handling can panic or resolve relative links from CWD. | **Fix:** safe parent-relative, cycle-checked traversal. |
| R05 | Clone parsing permits paths outside the repository base. | **Fix:** validate identity and canonical containment. |
| R06 | A failed `jj git clone` never falls back to Git. | **Fix:** retry only unavailable/unsupported-target failures with Git. |
| R07 | Repository and workflow commands are separate top-level commands. | **Fix:** nest them under `repo`; bare `repo` retains interactive selection. |
| R08 | Rust exposes `prune-branches`. | **Fix:** remove the command without a Swift replacement or alias. |
| R09 | Rust has no explicit spelling for the default `repo` selector behavior. | **Fix:** add equivalent `repo cd`. |
| S01 | Comma/newline shell records are ambiguous. | **Fix:** JSON Lines protocol. |
| S02 | Fish wrapper uses `eval` and loses CLI status. | **Fix:** parsed actions, argument-safe editor launch, preserved status. |
| S03 | Filesystem paths are converted with lossy UTF-8. | **Fix:** reject unrepresentable repository and action paths. |
| N01 | Daily notes use effective UTC. | **Fix:** user-local calendar. |
| N02 | Topic/person names permit traversal. | **Fix:** reject unsafe names. |
| T01 | Todo descriptions can make their own file unparsable. | **Fix:** reject empty/newline/control input. |
| T02 | Todo save assumes directory exists and uses shared temp name. | **Fix:** create directory and use unique atomic replacement. |
| U01 | Empty selectors and empty normal-mode buffers can underflow. | **Fix:** normal errors/no-op editing. |
| U02 | Prompt indexing mixes bytes, characters, and columns. | **Fix:** grapheme-safe editing/rendering. |
| U03 | Prompt failures can leave raw mode/cursor state active. | **Fix:** unconditional cleanup guard. |
| U04 | Selector identity is recovered from display strings. | **Fix:** retain opaque values/IDs separately. |
| U05 | Untrusted option text can inject terminal controls. | **Fix:** sanitize display text. |
| U06 | Interactive utilities are top-level commands. | **Fix:** move them under required parent command `tui` without top-level aliases. |
| G01 | Rust GitHub subcommands use underscores. | **Fix:** use kebab-case without snake_case aliases. |
| G02 | Closed-unmerged PRs are displayed as `open`. | **Fix:** display `closed`. |
| G03 | Missing head repository falls back to base repository. | **Fix:** emit `head.label: null`. |
| G04 | Rust paginates all status-check rollup contexts as of `3ebe4fe3`. | **Preserve:** fetch every page and retain GitHub's total. |
| G05 | Missing PR number silently chooses first HEAD-associated PR. | **Preserve**. |
| G06 | Commit-associated PR lookup silently uses GitHub's first page. | **Fix:** paginate every page. |
| G07 | Notification `--since` is not locally validated. | **Fix:** require ISO 8601 before request. |
| J01 | Jira filter help claims a nonexistent configured default. | **Fix:** omitted ID means favourite selector. |
| J02 | Duplicate filter labels select the first match. | **Fix:** select by opaque filter ID. |
| J03 | No favourites emits JSON `error` with exit `0`. | **Preserve** for JSON compatibility. |
| J04 | Search assumes legacy `startAt`/`total` pagination. | **Fix:** support token pagination while preserving CLI limits. |
| J05 | ADF has media, table, list-order, and decision-state rendering defects. | **Fix** in Swift Markdown rendering. |
| J06 | Typed ADF JSON reserialization drops unknown provider data. | **Fix:** preserve the complete ADF subtree in JSON. |
| L01 | Missing generic provider panics and chat names web chat. | **Fix:** correct ordinary config error. |
| L02 | Perplexity assumes zero-based citations and unsafe Unicode offsets. | **Fix:** one-based, grapheme-safe extraction. |
| L03 | Perplexity raw citations concatenate with the answer. | **Fix:** explicit blank-line delimiter. |
| L04 | Perplexity response requires both stream and nonstream fields. | **Fix:** decode fields appropriate to each response. |
| L05 | Vertex IDs are expired preview IDs. | **Fix:** stable Gemini 2.5 IDs. |
| L06 | Generic grounded Vertex chat does not enable Search. | **Fix:** grounded requests enable Search. |
| L07 | Empty Vertex candidates/parts panic. | **Fix:** provider error. |
| L08 | Anthropic stream error events print but still succeed. | **Fix:** report nonzero failure. |
| O01 | Completion supports five shells that Swift Argument Parser may not generate. | **Preserve** bash, elvish, fish, PowerShell, and zsh. |
| O02 | Workflow child commands use shell strings. | **Preserve** `sh -c` because repository configs depend on shell syntax. |
| O03 | Rust logs contain spelling mistakes such as `Cloing`/`dafault`. | **Fix:** do not preserve log typos. |

Anything not listed as an intentional difference follows the Rust command contract. When exact
formatting is not specified here, semantic fields, channels, and ordering are compatibility
requirements; incidental Rust `Debug`, panic, allocator, filesystem enumeration, and terminal
cursor details are not.
