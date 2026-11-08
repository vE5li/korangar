# Permanent remote assets (opt-in)

This is an asset transport, not a game-server protocol or a claim of 20190530
playability. It requires the separate remoteclient2 immutable-snapshot endpoint
PR; the existing `/client/` loose-file route alone is not sufficient. Neither
change is deployed automatically.

## Operation

Set `KORANGAR_REMOTE_BASE_URL=https://ro.turoran.com/client/` only after the
snapshot endpoint is reviewed and available. Set `KORANGAR_REMOTE_CACHE` to a
private user-owned directory on a persistent disk. The default is
`.korangar-assets/` relative to the working directory and is ignored by git.
Never commit the cache, game archives, or a real asset snapshot.

For a development snapshot on rodata-dev port 3339, supply that test service's
base URL and explicitly set `KORANGAR_REMOTE_ALLOW_HTTP=1`. Production defaults
to HTTPS and certificate verification. Credentials, fragments, query strings,
redirects and base URLs without a trailing slash are rejected.

The actual game `GameFileLoader` preserves existing local archive priority,
then checks the permanent verified cache, then fetches missing assets. Its
Lua discovery includes manifest paths, and the pinned snapshot revision feeds
the derived archive/cache hash. Existing archive settings remain in effect;
remote assets do not cause missing configured GRF files to be skipped. Configure
`client/game_archives.ron` with only local archives/directories that actually
exist (keep Korangar's own `archive/` resources when present).

An initial manifest is downloaded once. After that the local manifest pins the
snapshot; startup does not contact the server to check for updates. The manifest
is first-writer-wins across competing startup processes. Selecting a different
snapshot is an intentional operation using a new cache directory, not automatic
replacement. The old cache is retained.

Each blob is addressed by SHA-256, size-bounded (100 MiB), hashed and length
checked before an exclusive temporary file is synced and atomically published.
The manifest is bounded at 64 MiB. There is no LRU, eviction, or cleanup API.
Interrupted or corrupt responses do not publish an object. In-process cache
misses are serialized, preventing duplicate downloads of the same object.

For a resumable complete download without launching the renderer:

```sh
cargo run -p korangar-loaders --example fetch_assets
```

The command consumes the same environment variables. Once all listed objects
are verified locally, it succeeds with the server turned off, even without an
explicit offline flag. `KORANGAR_REMOTE_OFFLINE=1` additionally forbids all HTTP
requests and requires an existing manifest. `is_complete()` reads and verifies
objects locally; filenames or an empty manifest never establish completion.
Corruption invalidates completion. The manifest hashes detect corruption, not
an untrusted origin; the configured HTTPS server remains the trust anchor.

Requests outside the manifest, absolute paths, traversal, hidden components,
percent escapes, drive prefixes and query/fragment characters are rejected.
Assets are written to digest-named object files, never user-controlled virtual
paths. Cache roots must be private and trusted; this is not a sandbox against
another process with write access to the same user's directories.

The blocking loader runs on existing synchronous/worker-thread paths, not a
Tokio async task. On-demand fetch can block a requesting worker for up to the
configured timeout; use the prefetch command before play when desired.

## Evidence

- `cargo test -p korangar-loaders`: 9 tests, including corruption, interrupted
  response, concurrent misses, redirect denial, symlink denial, path/manifest
  validation, explicit offline mode, and reopening after the server exits.
- `cargo check -p korangar --bin korangar`: passes, including the integrated game
  loader. Development environment requires NASM and Slang >= 2025.18.2.
- Cross-repository synthetic test: build a two-file GRF, generate a snapshot,
  serve it with the actual new remoteclient2 binary, run `fetch_assets`, terminate
  the server, then run the same command again in normal online configuration.
  Both commands pass; the cached raw bytes match both synthetic originals.
- No real Gravity artwork, accounts, gameplay connections or live service
  changes are part of these tests.
