use clap::Parser;
use korangar::{Client, NoHooks};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Synchronize the asset cache and exit.
    #[arg(long)]
    sync_cache: bool,
}

fn main() {
    let arguments = Arguments::parse();

    if let Some(mut client) = Client::init(arguments.sync_cache, NoHooks) {
        client.run();
    }
}
