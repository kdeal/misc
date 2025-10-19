# Configuration

## Ollama provider

Enable the Ollama chat provider by adding an `[ollama]` section to your `wkfl.toml` configuration file:

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

You can select the provider explicitly via the `chat_provider` key. When no provider is specified, wkfl will choose Ollama automatically whenever an `[ollama]` block is present. Model names fall back from `thinking → large → small`, so you only need to set the models you intend to use.
