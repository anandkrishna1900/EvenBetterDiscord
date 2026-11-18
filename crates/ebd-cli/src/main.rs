use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ebd_config::{Config, ConfigError, LogFormat, LogLevel};
use ebd_contract::{AdapterError, Capability};
use ebd_core::{AdapterManager, CompatibilityReport};
use ebd_mock_client::MockClient;

const USAGE: &str = "Usage: ebd-cli [demo|capabilities|self-check|config [--init]]\n\
Environment: EBD_CONFIG overrides the config file path.";

#[derive(Debug)]
enum CliError {
    /// Bad invocation: reported with exit code 2.
    Usage(String),
    /// A contract operation failed: reported with exit code 1.
    Adapter(AdapterError),
    /// Configuration could not be loaded or written: exit code 1.
    Config(ConfigError),
}

impl From<AdapterError> for CliError {
    fn from(error: AdapterError) -> Self {
        CliError::Adapter(error)
    }
}

impl From<ConfigError> for CliError {
    fn from(error: ConfigError) -> Self {
        CliError::Config(error)
    }
}

fn main() -> ExitCode {
    match run_cli(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Usage(message)) => {
            eprintln!("ebd-cli: {message}");
            ExitCode::from(2)
        }
        Err(CliError::Adapter(error)) => {
            eprintln!("ebd-cli: {error}");
            ExitCode::FAILURE
        }
        Err(CliError::Config(error)) => {
            eprintln!("ebd-cli: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli<I>(args: I) -> Result<(), CliError>
where
    I: IntoIterator<Item = String>,
{
    run_cli_with(args, None)
}

/// Runs the CLI, optionally pinning the config path (used by tests).
fn run_cli_with<I>(args: I, config_override: Option<PathBuf>) -> Result<(), CliError>
where
    I: IntoIterator<Item = String>,
{
    let config_path = match config_override {
        Some(path) => Some(path),
        None => match ebd_config::default_path() {
            Ok(path) => Some(path),
            // A missing config root is not fatal: run with defaults.
            Err(ConfigError::NoConfigDir) => None,
            Err(error) => return Err(error.into()),
        },
    };

    let config = match &config_path {
        Some(path) => Config::load(path)?,
        None => Config::default(),
    };
    init_logging(&config);

    if let Some(path) = config_path.as_deref().filter(|path| path.exists()) {
        tracing::info!(
            path = %path.display(),
            level = %config.logging.level,
            "loaded configuration"
        );
    }

    let mut args = args.into_iter();
    let command = args.next().unwrap_or_else(|| "demo".into());
    let rest = args.collect::<Vec<_>>();

    tracing::debug!(command = %command, "dispatching command");

    match command.as_str() {
        "-h" | "--help" | "help" => {
            reject_extra(&rest, "help")?;
            println!("{USAGE}");
            Ok(())
        }
        "demo" => {
            reject_extra(&rest, "demo")?;
            with_mock_adapter(demo)
        }
        "capabilities" => {
            reject_extra(&rest, "capabilities")?;
            with_mock_adapter(capabilities)
        }
        "self-check" => {
            reject_extra(&rest, "self-check")?;
            with_mock_adapter(self_check)
        }
        "config" => {
            let init = match rest.as_slice() {
                [] => false,
                [flag] if flag == "--init" => true,
                _ => {
                    return Err(CliError::Usage(format!(
                        "`config` accepts only `--init`\n{USAGE}"
                    )))
                }
            };

            let path = config_path.ok_or_else(|| {
                CliError::Usage(format!(
                    "no configuration path; set EBD_CONFIG to choose one\n{USAGE}"
                ))
            })?;
            show_config(&path, init)?;
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "unknown command `{other}`\n{USAGE}"
        ))),
    }
}

/// Rejects arguments for commands that do not take any.
fn reject_extra(args: &[String], command: &str) -> Result<(), CliError> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(CliError::Usage(format!(
            "`{command}` does not take arguments\n{USAGE}"
        )))
    }
}

/// Installs the global subscriber described by the config.
///
/// The level comes from the config file; `EBD_CONFIG` selects the file. Later
/// per-plugin logging scopes can build on the same subscriber.
fn init_logging(config: &Config) {
    let level = match config.logging.level {
        LogLevel::Error => tracing::Level::ERROR,
        LogLevel::Warn => tracing::Level::WARN,
        LogLevel::Info => tracing::Level::INFO,
        LogLevel::Debug => tracing::Level::DEBUG,
        LogLevel::Trace => tracing::Level::TRACE,
    };

    let builder = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_writer(std::io::stderr);

    let installed = match config.logging.format {
        LogFormat::Text => builder.try_init(),
        LogFormat::Json => builder.json().try_init(),
    };

    // Tests and embedding hosts may install a subscriber before calling in.
    if installed.is_err() {
        tracing::debug!("a global log subscriber is already installed");
    }
}

/// Prints or creates the config file.
fn show_config(path: &Path, init: bool) -> Result<(), ConfigError> {
    let config = Config::load(path)?;

    if init {
        if path.exists() {
            return Err(ConfigError::AlreadyExists(path.to_owned()));
        }
        config.save(path)?;
        println!("Wrote default configuration to {}", path.display());
        return Ok(());
    }

    let state = if path.exists() {
        "present"
    } else {
        "not created yet, defaults in use"
    };
    println!("Config file: {} ({state})", path.display());
    print!("{}", config.to_json()?);
    println!();
    Ok(())
}

/// Attaches the offline mock adapter and runs `command` against it.
fn with_mock_adapter(
    command: impl FnOnce(&AdapterManager) -> Result<(), AdapterError>,
) -> Result<(), CliError> {
    let mut manager = AdapterManager::new();
    manager.attach(Box::new(MockClient));
    command(&manager)?;
    Ok(())
}

fn demo(manager: &AdapterManager) -> Result<(), AdapterError> {
    let user = manager
        .adapter_for(Capability::ReadCurrentUser)?
        .current_user()?;
    println!("Welcome, {}!", user.username);

    for guild in manager.adapter_for(Capability::ReadGuilds)?.guilds()? {
        println!("Community: {}", guild.name);

        for channel in manager
            .adapter_for(Capability::ReadChannels)?
            .channels(&guild.id)?
        {
            println!("  #{}", channel.name);

            match manager.adapter_for(Capability::ReadMessages) {
                Ok(adapter) => {
                    for message in adapter.messages(&channel.id, 5)? {
                        println!("    <{}> {}", message.author_id, message.content);
                    }
                }
                Err(AdapterError::Unsupported(capability)) => {
                    println!("    (adapter does not declare {capability})");
                }
                Err(error) => return Err(error),
            }

            match manager.adapter_for(Capability::SendMessages) {
                Ok(adapter) => {
                    let message =
                        adapter.send_message(&channel.id, "EBD mock adapter is running!")?;
                    println!("  Sent mock message: {}", message.id);
                }
                Err(AdapterError::Unsupported(capability)) => {
                    println!("  (adapter does not declare {capability})");
                }
                Err(error) => return Err(error),
            }
        }
    }

    Ok(())
}

fn capabilities(manager: &AdapterManager) -> Result<(), AdapterError> {
    let report = manager.report().ok_or_else(no_adapter)?;

    println!(
        "Client: {} ({})",
        report.client.client_name, report.client.client_version
    );
    println!("Adapter version: {}", report.client.adapter_version);
    println!("Contract: {}", report.contract_version);

    for capability in Capability::ALL {
        let state = if report.supports(capability) {
            "yes"
        } else {
            "no "
        };
        println!("  [{state}] {capability}");
    }

    Ok(())
}

fn self_check(manager: &AdapterManager) -> Result<(), AdapterError> {
    let report = manager.report().ok_or_else(no_adapter)?;
    self_check_report(&report)
}

fn self_check_report(report: &CompatibilityReport) -> Result<(), AdapterError> {
    let missing = report.missing();

    // Only print OK once the check has actually passed.
    if missing.is_empty() {
        println!("EBD self-check: OK");
    }
    println!(
        "Client: {} {}",
        report.client.client_name, report.client.client_version
    );
    println!("Contract: {}", report.contract_version);
    println!(
        "Capabilities: {}/{}",
        report.capabilities.len(),
        Capability::ALL.len()
    );

    if missing.is_empty() {
        return Ok(());
    }

    let names = missing
        .iter()
        .map(Capability::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    println!("Missing: {names}");

    // Surface the first gap so an incomplete adapter fails the check.
    Err(AdapterError::Unsupported(missing[0]))
}

fn no_adapter() -> AdapterError {
    AdapterError::Operation("no client adapter is attached".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("ebd-cli-{}-{name}", std::process::id()))
            .join("config.toml")
    }

    fn run(args: &[&str]) -> Result<(), CliError> {
        run_cli_with(
            args.iter().map(|arg| (*arg).to_string()),
            Some(config_path("shared")),
        )
    }

    #[test]
    fn help_is_successful() {
        assert!(run(&["--help"]).is_ok());
        assert!(run(&["help"]).is_ok());
    }

    #[test]
    fn demos_and_reports_are_successful_against_the_mock() {
        assert!(run(&[]).is_ok());
        assert!(run(&["demo"]).is_ok());
        assert!(run(&["capabilities"]).is_ok());
        assert!(run(&["self-check"]).is_ok());
    }

    #[test]
    fn unknown_and_extra_arguments_are_usage_errors() {
        assert!(matches!(run(&["nope"]), Err(CliError::Usage(_))));
        assert!(matches!(run(&["demo", "extra"]), Err(CliError::Usage(_))));
        assert!(matches!(run(&["config", "extra"]), Err(CliError::Usage(_))));
    }

    #[test]
    fn config_is_read_only_until_init_writes_it() {
        let path = config_path("show");
        let _ = std::fs::remove_dir_all(path.parent().expect("parent"));

        assert!(run_cli_with(std::iter::once("config".to_string()), Some(path.clone())).is_ok());
        assert!(!path.exists(), "`config` must not write the file");

        assert!(run_cli_with(
            std::iter::once("config".to_string()).chain(std::iter::once("--init".to_string())),
            Some(path.clone())
        )
        .is_ok());
        assert_eq!(
            Config::load(&path).expect("written config"),
            Config::default()
        );

        assert!(matches!(
            run_cli_with(
                std::iter::once("config".to_string()).chain(std::iter::once("--init".to_string())),
                Some(path)
            ),
            Err(CliError::Config(ConfigError::AlreadyExists(_)))
        ));
    }

    fn report_with(capabilities: Vec<Capability>) -> CompatibilityReport {
        CompatibilityReport {
            client: ebd_contract::ClientInfo {
                client_name: "Test Client".into(),
                client_version: "0.0.0".into(),
                adapter_version: "0.0.0".into(),
            },
            contract_version: ebd_contract::CONTRACT_VERSION,
            capabilities,
        }
    }

    #[test]
    fn self_check_fails_on_the_first_missing_capability() {
        let report = report_with(vec![Capability::ReadCurrentUser]);

        assert_eq!(
            self_check_report(&report),
            Err(AdapterError::Unsupported(Capability::ReadGuilds))
        );
    }

    #[test]
    fn self_check_passes_only_with_every_capability() {
        let report = report_with(Capability::ALL.to_vec());

        assert_eq!(self_check_report(&report), Ok(()));
    }
}
