use ebd_core::AdapterManager;
use ebd_mock_client::MockClient;

fn main() {
    let command = std::env::args().nth(1).unwrap_or_else(|| "demo".into());
    let mut manager = AdapterManager::new();
    manager.attach(Box::new(MockClient));

    match command.as_str() {
        "self-check" => {
            let report = manager.report().expect("adapter attached");
            println!("EBD self-check: OK");
            println!("Client: {} {}", report.client.client_name, report.client.client_version);
            println!("Contract: {}", report.contract_version);
            println!("Capabilities: {}", report.capabilities.len());
        }
        "demo" => {
            let adapter = manager.adapter().expect("adapter attached");
            let user = adapter.current_user().expect("mock user");
            println!("Welcome, {}!", user.username);
            for guild in adapter.guilds().expect("mock guilds") {
                println!("Community: {}", guild.name);
                for channel in adapter.channels(&guild.id).expect("mock channels") {
                    println!("  #{}", channel.name);
                    let message = adapter.send_message(&channel.id, "EBD mock adapter is running!")
                        .expect("mock send");
                    println!("  Sent mock message: {}", message.id);
                }
            }
        }
        _ => {
            eprintln!("Usage: ebd-cli [demo|self-check]");
            std::process::exit(2);
        }
    }
}
