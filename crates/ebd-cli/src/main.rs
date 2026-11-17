use std::process::ExitCode;

use ebd_contract::{AdapterError, Capability};
use ebd_core::{AdapterManager, CompatibilityReport};
use ebd_mock_client::MockClient;

const USAGE: &str = "Usage: ebd-cli [demo|capabilities|self-check]";

#[derive(Debug)]
enum CliError {
    /// Bad invocation: reported with exit code 2.
    Usage(String),
    /// A contract operation failed: reported with exit code 1.
    Adapter(AdapterError),
}

impl From<AdapterError> for CliError {
    fn from(error: AdapterError) -> Self {
        CliError::Adapter(error)
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
    }
}

fn run_cli<I>(args: I) -> Result<(), CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let command = args.next().unwrap_or_else(|| "demo".into());

    if args.next().is_some() {
        return Err(CliError::Usage(format!(
            "expected a single command\n{USAGE}"
        )));
    }

    match command.as_str() {
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        "demo" => with_mock_adapter(demo),
        "capabilities" => with_mock_adapter(capabilities),
        "self-check" => with_mock_adapter(self_check),
        other => Err(CliError::Usage(format!(
            "unknown command `{other}`\n{USAGE}"
        ))),
    }
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

    fn run(args: &[&str]) -> Result<(), CliError> {
        run_cli(args.iter().map(|arg| (*arg).to_string()))
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
