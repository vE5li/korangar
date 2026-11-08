//! Explicit resumable prefetch. No game server connection or artwork bundling.
use korangar_loaders::remote::{PersistentAssets, RemoteConfig};
fn main() -> std::io::Result<()> {
    let base_url = std::env::var("KORANGAR_REMOTE_BASE_URL").map_err(|_| std::io::Error::other("set KORANGAR_REMOTE_BASE_URL"))?;
    let cache_dir = std::env::var_os("KORANGAR_REMOTE_CACHE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| ".korangar-assets".into());
    let flag = |name| std::env::var(name).is_ok_and(|v| v == "1");
    let assets = PersistentAssets::open(RemoteConfig {
        base_url,
        cache_dir,
        allow_http: flag("KORANGAR_REMOTE_ALLOW_HTTP"),
        offline: flag("KORANGAR_REMOTE_OFFLINE"),
    })?;
    assets.download_all()?;
    println!("Pinned asset snapshot is verified and complete locally.");
    Ok(())
}
