<div align="center">

# Codex Web Search

**Bring Codex web search to your terminal and your agents.**

Use your own Codex credentials to search the web and retrieve readable results. The project provides a CLI and a Rust library, so other agents can use it as a search tool without starting a Codex agent or a model conversation loop.

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)
![License](https://img.shields.io/badge/license-Apache--2.0-green)

[中文](README.md) · **English**

[Quick start](#quick-start) · [Usage examples](#usage-examples) · [Authentication](#authentication) · [CLI options](#cli-options) · [Rust library](#rust-library)

</div>

## Features

- **Ready-to-use CLI**: submit a query and get JSON output for scripts and applications.
- **Search and page reading**: search, open results, and find text through command JSON.
- **Search sessions**: reuse a session ID to keep server-side page references across calls.
- **Two authentication methods**: read an existing Codex authentication file or supply an access token through an environment variable.
- **Rust library**: build requests, call the service, and parse results directly from Rust.

> This project uses Codex's `alpha/search` endpoint and is not an official OpenAI SDK. Endpoint compatibility and available operations depend on the upstream service.

## Quick start

### 1. Install

You need a toolchain that supports **Rust edition 2024**. From the project directory, install the executable:

```sh
cargo install --path . --locked
```

Make sure Cargo's installation directory, usually `~/.cargo/bin`, is on your `PATH`. To build without installing, run `cargo build --release --locked` and use `./target/release/codex-web-search`.

### 2. Configure authentication

Point to an existing Codex authentication file:

```sh
export CODEX_AUTH_FILE="$HOME/.codex/auth.json"
```

The program does not discover authentication files automatically. For credentials stored only in an OS keychain, use the [access token method](#provide-an-access-token-directly).

### 3. Search

```sh
codex-web-search --query 'Shanghai electric vehicle subsidy policies'
```

Each invocation writes one JSON object to stdout. On failure, it exits with a nonzero status and writes the error to stderr, so callers can distinguish results from errors.

## Usage examples

### Search with domain filters

Use `--query` for a single search query. To set domain filters or response length, or to open a page, pass a `web.run` argument object through stdin:

```sh
# These commands assume CODEX_AUTH_FILE or CODEX_ACCESS_TOKEN is already set.
printf '%s\n' '{"search_query":[{"q":"Shanghai electric vehicle subsidy eligibility","domains":["shanghai.gov.cn"]}],"response_length":"short"}' |
  codex-web-search --session-id shanghai-ev-policy
```

### Open results and find text

Reference IDs in search results belong to the server-side search session. Reuse the same `--session-id` when opening a result or finding text, and use a reference ID actually returned by the service:

```sh
# Replace turn0search0 with a reference ID from the previous result.
printf '%s\n' '{"open":[{"ref_id":"turn0search0"}]}' |
  codex-web-search --session-id shanghai-ev-policy

printf '%s\n' '{"find":[{"ref_id":"turn0search0","pattern":"subsidy"}]}' |
  codex-web-search --session-id shanghai-ev-policy
```

If you omit `--session-id`, the program generates a UUID and returns it in the output. Reuse that value for subsequent calls. The session ID associates server-side references; it does not save search history locally.

The complete command object is submitted to the service, so other supported operations, such as `click`, can also be used. Available commands and parameters depend on the upstream service.

### Integrate with another agent

Any application that can start a subprocess can call this tool. This Python example inherits authentication environment variables from its parent, submits a search, and reads the plaintext result:

```python
import json
import subprocess

result = subprocess.run(
    ["codex-web-search", "--session-id", "agent-research"],
    input=json.dumps({"search_query": [{"q": "Shanghai electric vehicle subsidy eligibility"}]}),
    text=True,
    capture_output=True,
    check=True,
)
response = json.loads(result.stdout)
print(response["output"])
```

Reuse `agent-research` for related searches and page operations within one research task; use a different ID for unrelated tasks. A failed invocation raises `subprocess.CalledProcessError`, with the error message available in its `stderr` attribute.

## Authentication

### Use a Codex authentication file

Pass the authentication file explicitly with `--auth-file`, or set an environment variable for subsequent calls:

```sh
export CODEX_AUTH_FILE="$HOME/.codex/auth.json"
codex-web-search --query 'Shanghai electric vehicle subsidy policies'
```

The program reads only `tokens.access_token` and the optional `tokens.account_id`. It does not modify the file. If you use a custom `CODEX_HOME`, specify the corresponding `auth.json` path; the program does not discover it automatically. Credentials stored only in an OS keychain must be supplied through the token method below.

### Provide an access token directly

Set `CODEX_ACCESS_TOKEN` rather than passing a token as a command-line argument. For example, in Bash:

```sh
unset CODEX_AUTH_FILE
read -rs -p 'Access token: ' CODEX_ACCESS_TOKEN
printf '\n'
export CODEX_ACCESS_TOKEN
codex-web-search --query 'Shanghai electric vehicle subsidy policies'
```

Use the **access token**, not an `id_token` or `refresh_token`. When `--auth-file` or `CODEX_AUTH_FILE` is set, file authentication takes precedence.

In token mode, `--account-id` or `CHATGPT_ACCOUNT_ID` can provide an account/workspace routing hint. Otherwise, the program tries to extract `chatgpt_account_id` from the JWT. The server still validates the token and its permissions. Do not set the account ID option or environment variable when using file authentication.

This project does not perform login or refresh tokens automatically. If the service returns HTTP 401, supply an updated token or sign in through Codex again before reading its authentication file. OpenAI API key login, OS keychain access, and personal access token metadata lookup are not implemented.

## Response format

Illustrative output:

```json
{
  "session_id": "shanghai-ev-policy",
  "output": "Readable search results, possibly including page references and sources...",
  "results": [],
  "encrypted_output": "..."
}
```

| Field | Meaning |
| --- | --- |
| `session_id` | The search session ID to reuse for subsequent page operations. |
| `output` | Plaintext results that can be read or passed to an agent. |
| `results` | Optional structured results whose contents are defined by the service. |
| `encrypted_output` | An optional opaque encrypted payload, passed through unchanged. |

The program does not decrypt `encrypted_output` or automatically submit it in subsequent requests. Agent integrations will usually use `output`. Pasting the encrypted string into a prompt as ordinary text is not equivalent to submitting a dedicated encrypted content item to a model service.

## CLI options

| Option | Description | Default |
| --- | --- | --- |
| `--query` | A single search query; otherwise read command JSON from stdin. | None |
| `--auth-file` | Codex authentication file; also configurable with `CODEX_AUTH_FILE`. | None |
| `--account-id` | Account/workspace hint in token mode; also configurable with `CHATGPT_ACCOUNT_ID`. | Extract from token if possible |
| `--session-id` | Associate search, open, and find requests. | Generated UUID |
| `--model` | Model name submitted to the search service. | `gpt-6-astra` |
| `--mode` | `live`, `cached`, or `indexed`. | `live` |


`live` allows live web access, `cached` requests cached access, and `indexed` requests indexed access. Actual behavior and model availability depend on the search service.


## Endpoint and implementation limits

This project calls the `alpha/search` endpoint used by Codex, rather than the hosted `web_search` tool in the public Responses API. It is not an official OpenAI SDK and does not guarantee future endpoint compatibility. Account permissions, availability, and billing are controlled by the upstream service. The project does not report per-search credit usage, and public API search prices should not be assumed to apply.


## License

The project declares the **Apache-2.0** license in `Cargo.toml`.
