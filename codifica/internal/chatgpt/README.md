# ChatGPT subscription proxy

This is a Linux-only, single-user, local Responses API proxy. It is separate from the web
server and forward proxy. It uses OAuth subscription credentials, not an OpenAI
API key. Account eligibility, available models, rate limits, and billing rules
are determined by OpenAI; signing in does not guarantee access to every model
or entitlement. In particular, ordinary API access and Codex subscription access
are different products. The direct flow requires the direct-token entitlement.

## Sign in and make a request

```sh
go run ./cmd/openaiproxy login
# Open the printed URL in a browser. The command waits for the callback.
go run ./cmd/openaiproxy serve
```

The default provider is `openai-codex`. With the serving process running:

```sh
curl -N http://127.0.0.1:8083/v1/responses \
  -H 'Content-Type: application/json' \
  -d '{"model":"gpt-5.1-codex","instructions":"Be concise.","input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"Say hello."}]}]}'
```

Choose a model available to your subscription. The example uses Responses message
items rather than shorthand string input; the proxy preserves those items. The
exact example still needs a live smoke test with an eligible account (see below).
Codex is streaming-only: the
proxy sets `stream: true` and `store: false`, and supplies empty `instructions`
when omitted. It returns the upstream SSE stream unchanged. It does not translate
Chat Completions or buffer SSE into a non-streaming response. Other unsupported
request parameters are left for the upstream to reject.

For the direct subscription flow instead:

```sh
go run ./cmd/openaiproxy login -provider openai
go run ./cmd/openaiproxy serve -provider openai
```

Both commands accept `-credentials /private/path/auth.json`. Credentials default
to `$XDG_CONFIG_HOME/codifica/auth.json`, or `~/.config/codifica/auth.json` when
`XDG_CONFIG_HOME` is unset, not Pi's file. Existing `~/.codifica/auth.json` files
are not migrated automatically; move the file after stopping all proxy processes
or select it explicitly with `-credentials`. Access tokens, refresh tokens, issued
client IDs, scopes, expiry, and a stable random host ID are stored there. File
replacement is atomic with mode `0600`; newly created directories have mode
`0700`. Keep the enclosing directory private. Login and serving commands acquire
an exclusive OS advisory lock **before loading** the store and hold it for their
entire lifetime. A second command using that file fails rather than overwriting a
stale snapshot. Stop serving before logging in again. Symlink paths are resolved
to the same canonical credential path. Paths containing `..` components are
rejected rather than cleaned before symlink resolution, which could otherwise
select a different store. Do not create hard-linked aliases or modify the file
with tools that do not honor the lock.

The `<credentials>.lock` sidecar is intentionally retained. Do not remove it while
a command is running: the OS releases the lock automatically on exit, including
crashes. The command and credential locking implementation are built for Linux
only; no cross-platform fallback is provided.
To sign out, stop the process and remove the credential JSON file (this removes
both providers; it does not revoke tokens at OpenAI).

Browser login binds `127.0.0.1:1455` for at most ten minutes. If the port is busy,
or the browser is on another machine, use `login -paste`: open the printed URL
and paste the **complete** final redirect URL into the terminal. The browser may
show a connection error because there is no callback listener in paste mode;
the redirect URL still contains the authorization result. Treat this URL as
sensitive. Both modes validate state, callback location, and grant shape before
exchanging a code. The listener validates the actual callback Host and does not
consume the login on malformed requests; well-formed provider denials end login. Codex device-code login is not implemented.

## Authentication trace

| Step | `openai` | `openai-codex` |
| --- | --- | --- |
| Browser authorization | `https://auth.openai.com/api/accounts/authorize` | `https://auth.openai.com/oauth/authorize` |
| Client | Starts with `dynamic_agent_client`; uses the issued `client_id` from the callback for token exchange and refresh | Fixed Codex client ID |
| Token exchange and refresh | `https://auth.openai.com/api/accounts/oauth/token` | `https://auth.openai.com/oauth/token` |
| Resource | `https://api.openai.com/v1` in authorization, exchange, and refresh | No resource parameter |
| Browser redirect | `http://127.0.0.1:1455/auth/callback` | `http://127.0.0.1:1455/auth/callback` |
| First model request | `https://api.openai.com/v1/responses` | `https://chatgpt.com/backend-api/codex/responses` |
| Authentication headers | `Authorization: Bearer <access token>` | Same bearer header, plus `chatgpt-account-id` extracted from the access token's JWT payload |

Both flows use PKCE S256 and a random state, request offline access, exchange an
authorization code with the verifier, and persist access/refresh credentials.
Direct authorization also includes a nonce and the
`chatgpt.tokens.use.direct` scope. Unlike Pi's `apiKey` abstraction, this proxy
puts the resulting OAuth access token directly in the bearer header.

Before a model request, the manager checks expiry. Within one minute of expiry,
it posts a refresh-token grant to the selected provider's token endpoint with
that provider's client ID (and resource for `openai`). Refreshes are serialized,
and rotated refresh tokens are persisted before the access token is used. Once a
refresh grant starts, it uses an independent 30-second context: a caller disconnect
cannot interrupt retrieval/persistence of the replacement token. Shutdown drains
an in-flight refresh and retries any pending credential save before releasing the
credential lock; an unresolved save failure makes the command exit with an error.
Model requests remain
tied to caller cancellation. Network failures, refresh deadlines, or process death
after the server consumes a token can still require re-login; no client can
eliminate that ambiguity. A refresh response without a new refresh token retains
the old one. Failed refresh
or persistence stops the model request; there is no API-key fallback. Upstream
401 responses are returned rather than replaying a potentially billable request;
log in again if credentials were revoked before their stated expiry.

The Codex JWT payload is decoded only for routing metadata, not trusted for local
authentication. OpenAI verifies the bearer token. Provider tokens are never
interchanged between the two backends.

## Security and limitations

- `serve -listen` defaults to `127.0.0.1:8083` and accepts only literal loopback
  addresses. There is **no local client authentication**: any local process able
  to connect can spend your subscription quota. Do not expose it through another
  proxy or tunnel without adding authentication and access control.
- Browser requests carrying an Origin header are rejected, JSON content type is
  required, and CORS is not enabled. This is not a multi-user web login system.
- Only `POST /v1/responses` and `GET /healthz` are exposed. The caller cannot
  select an upstream host. Caller headers/cookies and upstream cookies are not
  forwarded. Requests are limited to 8 MiB.
- OAuth clients refuse redirects and use a 30-second timeout. Model requests
  preserve cancellation and stream without a whole-request timeout. Tokens and
  raw OAuth error responses are not logged.
- Direct requests advertise JSON or SSE according to `stream`; Codex always
  advertises SSE. Unexpected upstream read errors abort the downstream connection
  instead of presenting a truncated response as successful EOF. A syntactically
  complete SSE stream that omits a terminal event is not validated by this proxy.
- No device-code login, automatic browser launch, credential encryption/keychain,
  or chat UI is included. This intentionally adds a usable subscription sign-in
  and request path, not a full SDK.

## Protocol fixtures and live verification

`testdata/openai.json` and `testdata/openai-codex.json` pin literal authorization
queries, redirect URIs, token endpoints, and exchange/refresh forms independently
of production constants. The direct contract follows the supplied Pi source
excerpts, including the issued client ID and resource on both grants. The Codex
`127.0.0.1` redirect is also used by
[official Codex browser login source, revision d257beb](https://github.com/openai/codex/blob/d257beb16a02e8bc218619d8bbc5105064cb252c/codex-rs/login/src/server.rs#L197).
There is no source evidence here requiring a change to `localhost`, so both
flows retain their explicit `127.0.0.1` redirects. These hosts are **not** treated
as interchangeable during callback validation.

Tests cover grant wire fixtures, caller cancellation during token rotation,
credential permissions and file locking, failed persistence, callback validation
and complete browser acknowledgment, provider headers, direct JSON responses, SSE
streaming, and broken-upstream connection termination. Mocked tests and source
inspection do **not** establish live provider acceptance. Before relying on each
provider, an eligible account must complete this manual smoke check:

1. Run `login -provider <provider>` and confirm the browser callback and CLI both
   finish successfully (also check `-paste` if needed).
2. Run `serve -provider <provider>` and issue a request with a model available to
   that account. For Codex, use the message-item example above and verify a
   `response.completed` event. For direct access, verify both JSON (`stream:false`)
   and SSE (`stream:true`) responses.
3. After the saved token enters its refresh window, make another request and
   confirm it succeeds and credential expiry/rotation is persisted, without
   printing tokens. Stop serving before inspecting the private credential file.

Live sign-in and these smoke checks have not been performed in this environment.
