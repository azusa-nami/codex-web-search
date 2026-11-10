use super::Cli;
use super::Mode;
use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use pretty_assertions::assert_eq;

#[test]
fn help_describes_the_standalone_interface() {
    let help = Cli::command().render_long_help().to_string();
    insta::assert_snapshot!("help", help);
}

#[test]
fn parses_search_and_reference_invocations() -> Result<()> {
    let cli = Cli::try_parse_from([
        "codex-web-search",
        "--query",
        "Rust",
        "--session-id",
        "research",
        "--mode",
        "cached",
    ])?;
    assert_eq!(
        (cli.query, cli.session_id),
        (Some("Rust".into()), Some("research".into()))
    );
    assert!(matches!(cli.mode, Mode::Cached));
    let cli = Cli::try_parse_from(["codex-web-search", "--session-id", "research"])?;
    assert!(cli.query.is_none());
    Ok(())
}
