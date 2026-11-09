//! Opt-in native Windows portable deployment. Never mutates environment
//! variables.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use korangar_loaders::remote::RemoteConfig;

const TEST_ASSETS: &str = "https://ro.turoran.com/client/";
static PORTABLE: OnceLock<PortablePaths> = OnceLock::new();
#[derive(Clone, Debug)]
struct PortablePaths {
    root: PathBuf,
    runtime: PathBuf,
}
#[cfg(any(windows, test))]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Deployment {
    profile: Profile,
    runtime: PathBuf,
}
#[cfg(any(windows, test))]
#[derive(serde::Deserialize)]
enum Profile {
    TuroranTest20250716,
}
#[cfg(any(windows, test))]
impl Default for Deployment {
    fn default() -> Self {
        Self {
            profile: Profile::TuroranTest20250716,
            runtime: "runtime".into(),
        }
    }
}

#[cfg(any(windows, test))]
fn resolve_paths(executable: &Path, runtime: &Path) -> Result<PortablePaths, String> {
    use std::path::Component;
    if runtime.as_os_str().is_empty() || runtime.components().any(|p| !matches!(p, Component::Normal(_))) {
        return Err("Portable runtime must be a nonempty relative child path without traversal".into());
    }
    let root = executable
        .parent()
        .ok_or("Executable has no parent directory")?
        .canonicalize()
        .map_err(|_| "Cannot resolve executable directory")?;
    let runtime = root
        .join(runtime)
        .canonicalize()
        .map_err(|_| "Portable runtime is missing; retain runtime/ and bundled/ beside korangar.exe")?;
    if !runtime.starts_with(&root) || runtime == root || !runtime.is_dir() {
        return Err("Portable runtime must remain inside the executable installation directory".into());
    }
    if !root.join("bundled").is_dir() || !runtime.join("client/game_archives.ron").is_file() {
        return Err("Missing portable resources: bundled/ or runtime/client/game_archives.ron; settings were not rewritten".into());
    }
    validate_test_clientinfo(&runtime.join("archive/data/sclientinfo.xml"))?;
    Ok(PortablePaths { root, runtime })
}
#[cfg(any(windows, test))]
fn validate_test_clientinfo(path: &Path) -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct Info {
        #[serde(default)]
        connection: Vec<Connection>,
    }
    #[derive(serde::Deserialize)]
    struct Connection {
        address: String,
        port: u16,
        packet_version: u32,
    }
    let text = std::fs::read_to_string(path).map_err(|_| "Missing TEST configuration: runtime/archive/data/sclientinfo.xml")?;
    let info: Info = quick_xml::de::from_str(&text).map_err(|_| "Invalid TEST clientinfo XML or missing explicit packet_version")?;
    if info.connection.len() != 1
        || !info
            .connection
            .iter()
            .all(|c| c.address == "192.168.11.110" && c.port == 6910 && c.packet_version == 20250716)
    {
        return Err(
            "TuroranTest20250716 requires TEST 192.168.11.110:6910 and packet_version 20250716; configuration was not rewritten".into(),
        );
    }
    Ok(())
}
/// Only Windows deployments with a sidecar opt in; Linux/developer launches are
/// unchanged.
pub fn prepare(interactive: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|_| "Cannot locate korangar.exe")?;
        let sidecar = exe.parent().ok_or("Executable has no directory")?.join("korangar-portable.ron");
        let config: Deployment = if sidecar.try_exists().map_err(|_| "Cannot inspect korangar-portable.ron")? {
            let text = std::fs::read_to_string(sidecar).map_err(|_| "Cannot read korangar-portable.ron")?;
            ron::from_str(&text)
                .map_err(|_| "Invalid korangar-portable.ron; expected (profile: TuroranTest20250716, runtime: \"runtime\")")?
        } else if cfg!(feature = "windows-test-deployment") {
            Deployment::default()
        } else {
            return Ok(());
        };
        let Profile::TuroranTest20250716 = config.profile;
        let paths = resolve_paths(&exe, &config.runtime)?;
        std::env::set_current_dir(&paths.runtime).map_err(|_| "Cannot enter portable runtime directory")?;
        PORTABLE.set(paths).map_err(|_| "Portable startup already initialized")?;
        std::panic::set_hook(Box::new(move |info| {
            // Panic payloads may contain entered text or packet contents: never persist
            // them.
            let location = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "unknown location".into());
            report_failure(
                &format!("Client panic at {location}; payload and credentials not recorded"),
                interactive,
            );
        }));
    }
    #[cfg(not(windows))]
    let _ = interactive;
    Ok(())
}
#[derive(Default)]
struct RemoteOverrides {
    base: Option<String>,
    cache: Option<PathBuf>,
    offline: Option<String>,
    allow_http: Option<String>,
}
fn resolve_remote(paths: Option<&PortablePaths>, overrides: RemoteOverrides) -> Option<RemoteConfig> {
    let base_url = overrides.base.or_else(|| paths.map(|_| TEST_ASSETS.to_owned()))?;
    let cache_dir = match (overrides.cache, paths) {
        (Some(p), Some(paths)) if p.is_relative() => paths.runtime.join(p),
        (Some(p), _) => p,
        (None, Some(paths)) => paths.runtime.join("asset-cache"),
        (None, None) => ".korangar-assets".into(),
    };
    Some(RemoteConfig {
        base_url,
        cache_dir,
        offline: overrides.offline.map_or(paths.is_some(), |v| v == "1"),
        allow_http: overrides.allow_http.is_some_and(|v| v == "1"),
    })
}
pub(crate) fn remote_config() -> Option<RemoteConfig> {
    resolve_remote(PORTABLE.get(), RemoteOverrides {
        base: std::env::var("KORANGAR_REMOTE_BASE_URL").ok(),
        cache: std::env::var_os("KORANGAR_REMOTE_CACHE").map(PathBuf::from),
        offline: std::env::var("KORANGAR_REMOTE_OFFLINE").ok(),
        allow_http: std::env::var("KORANGAR_REMOTE_ALLOW_HTTP").ok(),
    })
}
pub(crate) fn default_backends() -> wgpu::Backends {
    if PORTABLE.get().is_some() {
        wgpu::Backends::VULKAN
    } else {
        wgpu::Backends::all()
    }
}
/// Call only with sanitized technical summaries, never packet, clipboard or URL
/// values.
pub fn report_failure(summary: &str, interactive: bool) {
    use std::io::Write;
    #[cfg(not(windows))]
    if PORTABLE.get().is_none() {
        return;
    }
    let root = PORTABLE
        .get()
        .map(|p| p.root.clone())
        .or_else(|| std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)));
    if let Some(root) = root {
        let log = root.join("startup-error.log");
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log) {
            let _ = writeln!(file, "{summary}");
        }
        eprintln!("{summary}\nStartup diagnostics: {}", log.display());
        #[cfg(windows)]
        if interactive {
            show_error(&format!(
                "{summary}\n\nStartup diagnostics: {}\nKeep runtime/, bundled/ and runtime DLLs beside korangar.exe.",
                log.display()
            ));
        }
    }
    #[cfg(not(windows))]
    let _ = interactive;
}
#[cfg(windows)]
fn show_error(message: &str) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(window: *mut std::ffi::c_void, text: *const u16, title: *const u16, flags: u32) -> i32;
    }
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "Korangar startup failed".encode_utf16().chain(Some(0)).collect();
    // SAFETY: both NUL-terminated buffers live for the synchronous call; no owner
    // window. Headless modes never display dialogs.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x10);
    }
}
#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "korangar portable spaces {} {}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            for child in ["bundled", "runtime/client", "runtime/archive/data"] {
                std::fs::create_dir_all(root.join(child)).unwrap();
            }
            std::fs::write(root.join("runtime/client/game_archives.ron"), "(archives: [])").unwrap();
            std::fs::write(
                root.join("runtime/archive/data/sclientinfo.xml"),
                "<clientinfo><connection><address>192.168.11.110</address><port>6910</port><packet_version>20250716</packet_version></\
                 connection></clientinfo>",
            )
            .unwrap();
            Self(root)
        }

        fn paths(&self) -> Result<PortablePaths, String> {
            resolve_paths(&self.0.join("korangar.exe"), Path::new("runtime"))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn paths_are_executable_relative_and_preserve_spaces() {
        let f = Fixture::new();
        let p = f.paths().unwrap();
        assert_eq!(p.runtime, f.0.join("runtime").canonicalize().unwrap());
        assert_eq!(p.root, f.0.canonicalize().unwrap());
    }
    #[test]
    fn missing_resources_error_without_rewriting_settings() {
        let f = Fixture::new();
        let p = f.0.join("runtime/client/game_archives.ron");
        std::fs::remove_file(&p).unwrap();
        assert!(f.paths().unwrap_err().contains("game_archives.ron"));
        assert!(!p.exists());
    }
    #[test]
    fn runtime_traversal_and_absolute_paths_rejected() {
        let f = Fixture::new();
        for p in [Path::new("../runtime"), Path::new(""), f.0.as_path()] {
            assert!(resolve_paths(&f.0.join("korangar.exe"), p).is_err());
        }
    }
    #[test]
    fn test_version_and_endpoint_required_without_rewrites() {
        let f = Fixture::new();
        let p = f.0.join("runtime/archive/data/sclientinfo.xml");
        let text = std::fs::read_to_string(&p).unwrap().replace("20250716", "20220406");
        std::fs::write(&p, &text).unwrap();
        assert!(f.paths().is_err());
        assert_eq!(std::fs::read_to_string(p).unwrap(), text);
    }
    #[test]
    fn portable_remote_defaults_are_offline_install_relative() {
        let f = Fixture::new();
        let p = f.paths().unwrap();
        let c = resolve_remote(Some(&p), RemoteOverrides::default()).unwrap();
        assert!(c.offline);
        assert!(!c.allow_http);
        assert_eq!(c.base_url, TEST_ASSETS);
        assert_eq!(c.cache_dir, p.runtime.join("asset-cache"));
    }
    #[test]
    fn explicit_overrides_win_without_environment_mutation() {
        let f = Fixture::new();
        let p = f.paths().unwrap();
        let c = resolve_remote(Some(&p), RemoteOverrides {
            base: Some("https://example.invalid/assets/".into()),
            cache: Some("custom cache".into()),
            offline: Some("0".into()),
            allow_http: None,
        })
        .unwrap();
        assert_eq!(c.base_url, "https://example.invalid/assets/");
        assert!(!c.offline);
        assert_eq!(c.cache_dir, p.runtime.join("custom cache"));
        let absolute = p.root.join("explicit cache");
        let c = resolve_remote(Some(&p), RemoteOverrides {
            cache: Some(absolute.clone()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(c.cache_dir, absolute);
    }
    #[test]
    fn legacy_nonportable_remote_behavior_preserved() {
        assert!(resolve_remote(None, RemoteOverrides::default()).is_none());
        let c = resolve_remote(None, RemoteOverrides {
            base: Some("https://example.invalid/".into()),
            ..Default::default()
        })
        .unwrap();
        assert!(!c.offline);
        assert_eq!(c.cache_dir, Path::new(".korangar-assets"));
    }
    #[test]
    fn typed_profile_rejects_unknown_configuration() {
        let good: Deployment = ron::from_str("(profile: TuroranTest20250716, runtime: \"runtime\")").unwrap();
        assert!(matches!(good.profile, Profile::TuroranTest20250716));
        assert_eq!(good.runtime, Path::new("runtime"));
        assert!(ron::from_str::<Deployment>("(profile: Live, runtime: \"runtime\")").is_err());
        assert!(ron::from_str::<Deployment>("(profile: TuroranTest20250716, runtime: \"runtime\", password: \"synthetic\")").is_err());
    }
    #[test]
    fn compiled_test_defaults_need_no_sidecar_or_environment() {
        let config = Deployment::default();
        assert!(matches!(config.profile, Profile::TuroranTest20250716));
        assert_eq!(config.runtime, Path::new("runtime"));
    }
    #[cfg(unix)]
    #[test]
    fn runtime_symlink_cannot_escape_installation() {
        let fixture = Fixture::new();
        std::os::unix::fs::symlink(std::env::temp_dir(), fixture.0.join("outside")).unwrap();
        assert!(resolve_paths(&fixture.0.join("korangar.exe"), Path::new("outside")).is_err());
    }
}
