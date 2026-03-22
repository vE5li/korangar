use korangar::Client;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sync_cache = args.len() > 1 && &args[1] == "sync-cache";

    if let Some(mut client) = Client::init(sync_cache) {
        client.run();
    }
}
