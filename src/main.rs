use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use clap::Parser;
use clap::ValueEnum;
use codex_web_search::SearchAuth;
use codex_web_search::SearchClient;
use codex_web_search::SearchMode;
use codex_web_search::SearchRequest;
use std::io::Read;
use std::path::PathBuf;

#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    Live,
    Cached,
    Indexed,
}

#[derive(Parser)]
#[command(
    about = "Search using a caller-provided Codex access token. Reads web.run JSON from stdin unless --query is provided."
)]
struct Cli {
    /// Read tokens from this Codex auth.json instead of CODEX_ACCESS_TOKEN.
    #[arg(long, env = "CODEX_AUTH_FILE")]
    auth_file: Option<PathBuf>,
    /// Optional workspace routing hint; otherwise read from the token/auth file.
    #[arg(long, env = "CHATGPT_ACCOUNT_ID")]
    account_id: Option<String>,
    /// Search a single query instead of reading command JSON from stdin.
    #[arg(long)]
    query: Option<String>,
    /// Reuse this ID when opening a search result from an earlier invocation.
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long, default_value = "gpt-6-astra")]
    model: String,
    #[arg(long, value_enum, default_value = "live")]
    mode: Mode,
    #[arg(long, default_value_t = 10_000)]
    max_output_tokens: u64,
    #[arg(long, default_value = codex_web_search::DEFAULT_BASE_URL)]
    base_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let auth = if let Some(path) = cli.auth_file {
        ensure!(
            cli.account_id.is_none(),
            "--account-id is only used with CODEX_ACCESS_TOKEN"
        );
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(/*limit*/ 256 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        SearchAuth::from_auth_json(&bytes)?
    } else {
        SearchAuth::from_access_token(
            &std::env::var("CODEX_ACCESS_TOKEN")
                .context("set CODEX_ACCESS_TOKEN or use --auth-file /path/to/auth.json")?,
            cli.account_id.as_deref(),
        )?
    };
    let commands = if let Some(query) = cli.query {
        ensure!(!query.trim().is_empty(), "query is empty");
        serde_json::json!({"search_query":[{"q":query}]})
    } else {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(/*limit*/ 256 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= 256 * 1024, "commands exceed 256 KiB");
        serde_json::from_slice(&bytes).context("stdin must contain a web.run JSON object")?
    };
    let mut request = SearchRequest::new(
        cli.session_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        cli.model,
        commands,
    );
    request.max_output_tokens = cli.max_output_tokens;
    request.mode = match cli.mode {
        Mode::Cached => SearchMode::Cached,
        Mode::Indexed => SearchMode::Indexed,
        Mode::Live => SearchMode::Live,
    };
    let response = SearchClient::with_base_url(auth, &cli.base_url)?
        .search(&request)
        .await?;
    let mut output = serde_json::to_value(response)?;
    output["session_id"] = serde_json::json!(request.id);
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
