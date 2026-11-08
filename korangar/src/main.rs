use clap::Parser;
use korangar::Client;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Synchronize the asset cache and exit.
    #[arg(long)]
    sync_cache: bool,
    /// Check asset source and native clientinfo setup without a window/login.
    #[arg(long)]
    check_bootstrap: bool,
}

fn main() {
    let arguments = Arguments::parse();

    if arguments.check_bootstrap {
        match Client::check_bootstrap() {
            Ok(()) => {
                println!("Native bootstrap OK (configuration/source initialization only; no completeness, rendering or gameplay claim)")
            }
            Err(error) => {
                eprintln!("Native bootstrap failed: {error}");
                std::process::exit(2);
            }
        }
        return;
    }
    if let Some(mut client) = Client::init(arguments.sync_cache) {
        if let Err(error) = client.run() {
            eprintln!("Native client failed: {error}");
            std::process::exit(2);
        }
    } else if !arguments.sync_cache {
        std::process::exit(2);
    }
}
