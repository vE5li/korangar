//! Permanent content-addressed assets. No eviction, implicit update checks, or
//! URL-bearing error messages. Use from synchronous loader/worker threads, not
//! a Tokio async task (reqwest's blocking client owns a runtime).
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use reqwest::Url;
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_ASSET: u64 = 100 * 1024 * 1024;
const MAX_MANIFEST: u64 = 64 * 1024 * 1024;

/// One raw game asset, not a filesystem or network URL.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetEntry {
    /// Canonical lower-case virtual path using forward slashes.
    pub path: String,
    /// Exact uncompressed byte length.
    pub size: u64,
    /// Lower-case SHA-256 hex digest of raw bytes.
    pub sha256: String,
}

/// Immutable snapshot contract shared with remoteclient2's v1 endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Must equal one.
    pub format_version: u32,
    /// SHA-256 of compact JSON serialization of the sorted `files` array.
    pub revision: String,
    /// Strictly sorted unique entries; an empty set is never "complete".
    pub files: Vec<AssetEntry>,
}

fn error(message: &'static str) -> io::Error {
    io::Error::other(message)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_digest(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Canonicalize a virtual RO path without decoding percent escapes or accepting
/// filesystem roots, hidden components, query strings, drives, or traversal.
pub fn canonical_path(path: &str) -> Option<String> {
    if path.starts_with(['/', '\\']) || path.contains(['%', ':', '?', '#']) || path.chars().any(char::is_control) {
        return None;
    }
    let path = path.replace('\\', "/").to_lowercase();
    let mut parts = path.split('/');
    let root = parts.next()?;
    if !["data", "bgm", "system", "systemen", "ai"].contains(&root) {
        return None;
    }
    let rest: Vec<_> = parts.collect();
    if rest.is_empty() || rest.iter().any(|p| p.is_empty() || p.starts_with('.')) {
        return None;
    }
    Some(path)
}

impl Manifest {
    /// Validate a bounded v1 manifest, including canonical revision and paths.
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() as u64 > MAX_MANIFEST {
            return Err(error("manifest too large"));
        }
        let m: Self = serde_json::from_slice(bytes).map_err(|_| error("invalid asset manifest"))?;
        if m.format_version != 1
            || m.files.is_empty()
            || !valid_digest(&m.revision)
            || digest(&serde_json::to_vec(&m.files).map_err(|_| error("invalid asset entries"))?) != m.revision
        {
            return Err(error("invalid snapshot revision"));
        }
        let mut previous: Option<&str> = None;
        for e in &m.files {
            if canonical_path(&e.path).as_deref() != Some(&e.path)
                || !valid_digest(&e.sha256)
                || e.size > MAX_ASSET
                || previous.is_some_and(|p| p >= e.path.as_str())
            {
                return Err(error("invalid asset entry"));
            }
            previous = Some(&e.path);
        }
        let mut sizes = BTreeMap::new();
        for e in &m.files {
            if let Some(old) = sizes.insert(&e.sha256, e.size) {
                if old != e.size {
                    return Err(error("inconsistent asset size"));
                }
            }
        }
        Ok(m)
    }
}

/// Explicit connection and cache configuration. A cache directory pins one
/// snapshot forever; use a new directory for an intentional snapshot update.
pub struct RemoteConfig {
    /// Trusted remoteclient base including the reverse-proxy prefix, ending
    /// `/`.
    pub base_url: String,
    /// Private user-owned directory; never place it in an untrusted shared
    /// tree.
    pub cache_dir: PathBuf,
    /// Explicit development-only opt-in; production requires HTTPS.
    pub allow_http: bool,
    /// Refuse any HTTP requests, including the initial manifest download.
    pub offline: bool,
}

/// Verified permanent cache and on-demand downloader. Downloads are serialized
/// to deduplicate concurrent misses. Existing local content is always checked
/// before a request. Completion and reopening never contact the server.
pub struct PersistentAssets {
    config: RemoteConfig,
    base: Url,
    client: Client,
    manifest: Manifest,
    entries: BTreeMap<String, AssetEntry>,
    download_lock: Mutex<()>,
}

fn read_bounded(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(error("cache entry is not a regular file"));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(error("cache entry too large"));
    }
    Ok(bytes)
}
fn atomic_write(root: &Path, target: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut temp = tempfile::NamedTempFile::new_in(root)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(target).map_err(|e| e.error)?;
    #[cfg(unix)]
    fs::File::open(root)?.sync_all()?;
    Ok(())
}
fn fetch(client: &Client, url: Url, limit: u64) -> io::Result<Vec<u8>> {
    let response = client.get(url).send().map_err(|_| error("asset request failed"))?;
    if !response.status().is_success() {
        return Err(error("asset request was not successful"));
    }
    if response.content_length().is_some_and(|s| s > limit) {
        return Err(error("asset response too large"));
    }
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("asset response interrupted"))?;
    if bytes.len() as u64 > limit {
        return Err(error("asset response too large"));
    }
    Ok(bytes)
}
impl PersistentAssets {
    /// Open the pinned local manifest, or fetch and atomically persist it once.
    /// A corrupt existing manifest is an error, never silently replaced by a
    /// different server revision. No credentials may be embedded in the URL.
    pub fn open(config: RemoteConfig) -> io::Result<Self> {
        let base = Url::parse(&config.base_url).map_err(|_| error("invalid asset base URL"))?;
        if !(base.scheme() == "https" || (config.allow_http && base.scheme() == "http"))
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || !base.path().ends_with('/')
        {
            return Err(error("asset base must be a credential-free HTTPS directory URL"));
        }
        fs::create_dir_all(&config.cache_dir)?;
        if fs::symlink_metadata(&config.cache_dir)?.file_type().is_symlink() {
            return Err(error("symlink cache root"));
        }
        let objects = config.cache_dir.join("objects");
        fs::create_dir_all(&objects)?;
        if fs::symlink_metadata(&objects)?.file_type().is_symlink() {
            return Err(error("symlink object directory"));
        }
        let client = Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| error("asset HTTP initialization failed"))?;
        let manifest_path = config.cache_dir.join("manifest.json");
        let bytes = match read_bounded(&manifest_path, MAX_MANIFEST) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound && !config.offline => {
                let url = base.join("api/assets/v1/manifest").map_err(|_| error("invalid manifest URL"))?;
                let bytes = fetch(&client, url, MAX_MANIFEST)?;
                Manifest::parse(&bytes)?;
                let mut temp = tempfile::NamedTempFile::new_in(&config.cache_dir)?;
                temp.write_all(&bytes)?;
                temp.as_file().sync_all()?;
                match temp.persist_noclobber(&manifest_path) {
                    Ok(_) => {
                        #[cfg(unix)]
                        fs::File::open(&config.cache_dir)?.sync_all()?;
                        bytes
                    }
                    Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => read_bounded(&manifest_path, MAX_MANIFEST)?,
                    Err(e) => return Err(e.error),
                }
            }
            Err(e) => return Err(e),
        };
        let manifest = Manifest::parse(&bytes)?;
        let entries = manifest.files.iter().map(|e| (e.path.clone(), e.clone())).collect();
        Ok(Self {
            config,
            base,
            client,
            manifest,
            entries,
            download_lock: Mutex::new(()),
        })
    }

    /// Pinned revision used for the application's derived cache identity.
    pub fn revision(&self) -> &str {
        &self.manifest.revision
    }

    /// Paths advertised by the validated manifest, useful for Lua patch
    /// discovery.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// True only for a manifest-authorized canonical game path.
    pub fn contains(&self, path: &str) -> bool {
        canonical_path(path).is_some_and(|p| self.entries.contains_key(&p))
    }

    fn local(&self, e: &AssetEntry) -> io::Result<Vec<u8>> {
        let bytes = read_bounded(&self.config.cache_dir.join("objects").join(&e.sha256), e.size)?;
        if bytes.len() as u64 != e.size || digest(&bytes) != e.sha256 {
            return Err(error("asset integrity check failed"));
        }
        Ok(bytes)
    }

    /// Retrieve one file, permanently storing a verified response atomically.
    /// Failed/interrupted transfers never publish a final cache entry.
    pub fn get(&self, path: &str) -> io::Result<Vec<u8>> {
        let path = canonical_path(path).ok_or_else(|| error("invalid virtual asset path"))?;
        let entry = self.entries.get(&path).ok_or_else(|| error("asset not in pinned manifest"))?;
        if let Ok(bytes) = self.local(entry) {
            return Ok(bytes);
        }
        if self.config.offline {
            return Err(error("asset unavailable offline"));
        }
        let _guard = self.download_lock.lock().map_err(|_| error("asset lock poisoned"))?;
        if let Ok(bytes) = self.local(entry) {
            return Ok(bytes);
        }
        let url = self
            .base
            .join(&format!("api/assets/v1/blobs/{}", entry.sha256))
            .map_err(|_| error("invalid blob URL"))?;
        let bytes = fetch(&self.client, url, entry.size)?;
        if bytes.len() as u64 != entry.size || digest(&bytes) != entry.sha256 {
            return Err(error("downloaded asset integrity check failed"));
        }
        let objects = self.config.cache_dir.join("objects");
        atomic_write(&objects, &objects.join(&entry.sha256), &bytes)?;
        Ok(bytes)
    }

    /// Reverify every unique object locally. Never performs a network request.
    pub fn is_complete(&self) -> bool {
        let mut checked = BTreeSet::new();
        self.entries.values().all(|e| !checked.insert(&e.sha256) || self.local(e).is_ok())
    }

    /// Explicit full download, resumable across process restarts. Verified
    /// existing objects are not fetched again and no object is ever evicted.
    pub fn download_all(&self) -> io::Result<()> {
        for entry in self.entries.values() {
            self.get(&entry.path)?;
        }
        if !self.is_complete() {
            return Err(error("snapshot incomplete"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    fn manifest(content: &[u8]) -> Vec<u8> {
        let files = vec![AssetEntry {
            path: "data/test.spr".into(),
            size: content.len() as u64,
            sha256: digest(content),
        }];
        serde_json::to_vec(&Manifest {
            format_version: 1,
            revision: digest(&serde_json::to_vec(&files).unwrap()),
            files,
        })
        .unwrap()
    }
    fn server(replies: Vec<(String, Vec<u8>)>) -> (String, Arc<AtomicUsize>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/client/", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        let thread = std::thread::spawn(move || {
            for (expected, body) in replies {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut request = Vec::new();
                let mut byte = [0];
                while !request.ends_with(b"\r\n\r\n") {
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                assert!(String::from_utf8_lossy(&request).starts_with(&format!("GET /client/{expected} HTTP/1.1")));
                seen.fetch_add(1, Ordering::SeqCst);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        (base, count, thread)
    }
    fn config(base: String, cache: &Path) -> RemoteConfig {
        RemoteConfig {
            base_url: base,
            cache_dir: cache.into(),
            allow_http: true,
            offline: false,
        }
    }
    #[test]
    fn on_demand_persists_and_restarts_without_server() {
        let content = b"synthetic sprite";
        let hash = digest(content);
        let (base, count, server) = server(vec![
            ("api/assets/v1/manifest".into(), manifest(content)),
            (format!("api/assets/v1/blobs/{hash}"), content.to_vec()),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let assets = PersistentAssets::open(config(base.clone(), dir.path())).unwrap();
        assert!(!assets.is_complete());
        assert_eq!(assets.get("DATA\\test.spr").unwrap(), content);
        server.join().unwrap();
        assert!(assets.is_complete());
        assert_eq!(assets.get("data/test.spr").unwrap(), content);
        assert_eq!(count.load(Ordering::SeqCst), 2);
        drop(assets);
        // Listener is gone. Reopen in normal online mode without making even a manifest
        // request.
        let assets = PersistentAssets::open(config(base, dir.path())).unwrap();
        assets.download_all().unwrap();
        assert_eq!(assets.get("data/test.spr").unwrap(), content);
        fs::write(dir.path().join("objects").join(hash), b"corrupt").unwrap();
        assert!(!assets.is_complete());
    }
    #[test]
    fn hash_mismatch_is_not_published() {
        let hash = digest(b"correct");
        let (base, _, server) = server(vec![
            ("api/assets/v1/manifest".into(), manifest(b"correct")),
            (format!("api/assets/v1/blobs/{hash}"), b"invalid".to_vec()),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let assets = PersistentAssets::open(config(base, dir.path())).unwrap();
        assert!(assets.get("data/test.spr").is_err());
        server.join().unwrap();
        assert!(!dir.path().join("objects").join(hash).exists());
        assert!(!assets.is_complete());
    }
    #[test]
    fn manifest_paths_duplicates_revision_and_empty_sets_are_rejected() {
        for path in [
            ".env",
            "/data/a",
            "data/../a",
            "data//a",
            "data/%2e%2e/a",
            "data/.git/a",
            "C:/a",
            "data/a?b",
            "data/a\0",
        ] {
            assert!(canonical_path(path).is_none());
        }
        let mut m: Manifest = serde_json::from_slice(&manifest(b"x")).unwrap();
        m.files[0].path = "src/main.rs".into();
        m.revision = digest(&serde_json::to_vec(&m.files).unwrap());
        assert!(Manifest::parse(&serde_json::to_vec(&m).unwrap()).is_err());
        m.files.clear();
        m.revision = digest(b"[]");
        assert!(Manifest::parse(&serde_json::to_vec(&m).unwrap()).is_err());
        let mut m: Manifest = serde_json::from_slice(&manifest(b"x")).unwrap();
        m.files.push(m.files[0].clone());
        m.revision = digest(&serde_json::to_vec(&m.files).unwrap());
        assert!(Manifest::parse(&serde_json::to_vec(&m).unwrap()).is_err());
        let mut bytes = manifest(b"x");
        let n = bytes.len();
        bytes[n - 3] ^= 1;
        assert!(Manifest::parse(&bytes).is_err());
    }
    #[test]
    fn explicit_offline_mode_never_bootstraps_network() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = config("http://127.0.0.1:1/".into(), dir.path());
        c.offline = true;
        assert!(PersistentAssets::open(c).is_err());
        fs::write(dir.path().join("manifest.json"), manifest(b"x")).unwrap();
        let mut c = config("http://127.0.0.1:1/".into(), dir.path());
        c.offline = true;
        let assets = PersistentAssets::open(c).unwrap();
        assert!(assets.get("data/test.spr").is_err());
        assert!(!assets.is_complete());
    }
    #[test]
    fn insecure_or_credentialed_origins_require_rejection() {
        let dir = tempfile::tempdir().unwrap();
        for base in [
            "http://localhost/",
            "https://user:secret@example.invalid/",
            "file:///tmp/",
            "https://example.invalid/?token=x",
            "https://example.invalid/client",
        ] {
            let mut c = config(base.into(), dir.path());
            c.allow_http = false;
            assert!(PersistentAssets::open(c).is_err());
        }
    }
}
#[cfg(test)]
mod failure_tests {
    use std::net::TcpListener;
    use std::sync::Arc;

    use super::*;
    fn local_manifest(cache: &Path, bytes: &[u8]) {
        let files = vec![AssetEntry {
            path: "data/test.spr".into(),
            size: bytes.len() as u64,
            sha256: digest(bytes),
        }];
        let m = Manifest {
            format_version: 1,
            revision: digest(&serde_json::to_vec(&files).unwrap()),
            files,
        };
        fs::write(cache.join("manifest.json"), serde_json::to_vec(&m).unwrap()).unwrap();
    }
    #[test]
    fn interrupted_response_does_not_publish_partial_object() {
        let dir = tempfile::tempdir().unwrap();
        local_manifest(dir.path(), b"complete");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            s.read(&mut buf).unwrap();
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\npart")
                .unwrap();
        });
        let assets = PersistentAssets::open(RemoteConfig {
            base_url: base,
            cache_dir: dir.path().into(),
            allow_http: true,
            offline: false,
        })
        .unwrap();
        assert!(assets.get("data/test.spr").is_err());
        server.join().unwrap();
        assert!(!assets.is_complete());
        assert_eq!(fs::read_dir(dir.path().join("objects")).unwrap().count(), 0);
    }
    #[test]
    fn concurrent_cache_misses_download_only_once() {
        let dir = tempfile::tempdir().unwrap();
        local_manifest(dir.path(), b"complete");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            s.read(&mut buf).unwrap();
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\ncomplete")
                .unwrap();
        });
        let assets = Arc::new(
            PersistentAssets::open(RemoteConfig {
                base_url: base,
                cache_dir: dir.path().into(),
                allow_http: true,
                offline: false,
            })
            .unwrap(),
        );
        let jobs: Vec<_> = (0..8)
            .map(|_| {
                let a = assets.clone();
                std::thread::spawn(move || assert_eq!(a.get("data/test.spr").unwrap(), b"complete"))
            })
            .collect();
        for job in jobs {
            job.join().unwrap();
        }
        server.join().unwrap();
        assert!(assets.is_complete());
    }
    #[test]
    fn redirects_are_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            s.read(&mut buf).unwrap();
            s.write_all(b"HTTP/1.1 302 Found\r\nContent-Length: 0\r\nLocation: http://127.0.0.1:1/private\r\nConnection: close\r\n\r\n")
                .unwrap();
        });
        assert!(
            PersistentAssets::open(RemoteConfig {
                base_url: base,
                cache_dir: dir.path().into(),
                allow_http: true,
                offline: false
            })
            .is_err()
        );
        server.join().unwrap();
        assert!(!dir.path().join("manifest.json").exists());
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_accepted_as_verified_objects() {
        let dir = tempfile::tempdir().unwrap();
        local_manifest(dir.path(), b"complete");
        fs::create_dir(dir.path().join("objects")).unwrap();
        let external = tempfile::NamedTempFile::new().unwrap();
        fs::write(external.path(), b"complete").unwrap();
        std::os::unix::fs::symlink(external.path(), dir.path().join("objects").join(digest(b"complete"))).unwrap();
        let assets = PersistentAssets::open(RemoteConfig {
            base_url: "https://example.invalid/".into(),
            cache_dir: dir.path().into(),
            allow_http: false,
            offline: true,
        })
        .unwrap();
        assert!(!assets.is_complete());
        assert!(assets.get("data/test.spr").is_err());
    }
}
