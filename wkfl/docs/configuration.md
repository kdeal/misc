# Configuration

## Global configuration file

wkfl reads its primary settings from `~/.config/wkfl/config.toml`. Any value in this file can be
provided directly or via the secret resolution prefixes `env::MY_VAR`, `cmd::some command`, or
`val::literal` to pull secrets from the environment or shell commands.

### Repository and notes directories

```toml
# Default: "~/repos/"
repositories_directory = "~/repos/"

# Optional path for personal notes. Defaults to "<repositories_directory>/notes" when omitted.
notes_directory = "~/notes" 
```

### Chat provider selection

```toml
# Force a specific chat provider instead of relying on automatic detection.
# Valid options: "VertexAI", "Anthropic", "Ollama"
chat_provider = "Ollama"

# Force a specific web-grounded chat provider.
# Valid options: "VertexAI", "Perplexity"
web_chat_provider = "Perplexity"
```

wkfl automatically selects a provider when these fields are omitted:

- `chat_provider` falls back to the first configured of `anthropic_api_key`, `[vertex_ai]`, or
  `[ollama]`.
- `web_chat_provider` falls back to `perplexity_api_key` when set, otherwise `[vertex_ai]`.

### Providers

#### Ollama provider

Enable the Ollama chat provider by adding an `[ollama]` section to your `config.toml` file:

```toml
[ollama]
# Optional. Defaults to http://localhost:11434 when omitted or blank.
base_url = "http://localhost:11434"

# Required. The base model used for `ModelType::Small` requests.
small = "llama3.2:3b"

# Optional. Falls back to the value of `small` when not provided.
large = "llama3.1:8b"

# Optional. Falls back to the value of `large` when not provided.
thinking = "llama3.1:70b"
```

You can also select the provider explicitly via the `chat_provider` key. When no provider is
specified, wkfl will choose Ollama automatically whenever an `[ollama]` block is present. Model names
fall back from `thinking → large → small`, so you only need to set the models you intend to use.

#### Anthropic provider

Set the `anthropic_api_key` field to enable Anthropic chat:

```toml
# Accepts raw values or secret references like "env::ANTHROPIC_API_KEY".
anthropic_api_key = "env::ANTHROPIC_API_KEY"
```

#### Vertex AI provider

```toml
[vertex_ai]
# Supports secret resolution via env::/cmd::/val:: prefixes
api_key = "env::VERTEX_API_KEY"
project_id = "my-gcp-project"
```

#### Perplexity provider

```toml
# Supports secret resolution via env::/cmd::/val:: prefixes
perplexity_api_key = "env::PERPLEXITY_API_KEY"
```

### GitHub tokens

GitHub tokens are mapped by host inside the `[github_tokens]` table:

```toml
[github_tokens]
"github.com" = "env::GITHUB_TOKEN"
"github.mycompany.com" = "env::GHE_TOKEN"
```

### Jira configuration

```toml
[jira]
instance_url = "https://mycompany.atlassian.net"
email = "me@example.com"
# Supports secret resolution via env::/cmd::/val:: prefixes
api_token = "env::JIRA_API_TOKEN"
```

## Repository-specific configuration

wkfl looks for project-level settings in two locations (later entries override earlier ones):

1. `<repo>/.git/info/wkfl.toml`
2. `<repo>/.wkfl.toml`

These files currently support overriding workflow commands:

```toml
# Commands to run for `wkfl test`
test_commands = ["cargo test"]

# Commands to run for `wkfl fmt`
fmt_commands = ["cargo fmt"]

# Commands to run for `wkfl build`
build_commands = ["cargo build"]
```
