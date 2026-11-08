# Native Turoran TEST bootstrap (not a full game install)

Use only the isolated hercules-test account. Never put credentials in this
checkout, profile XML, screenshots, command arguments or logs. This profile
selects **20250716 MAIN** explicitly and connects to **192.168.11.110:6910**;
character/map endpoints are supplied by TEST (6131/5131). The browser asset
clientinfo (localhost:6900, version55) is not a native backend configuration.

From the checkout, stage a NEW private runtime outside the repository:

```sh
python3 scripts/stage-turoran-test.py --runtime /root/Work/korangar-native-test --sample-assets
. /root/Work/korangar-native-test/run-env.sh
/root/Work/korangar/target/debug/korangar --check-bootstrap
```

The generated archive configuration mounts the checkout's existing bundled UI/font assets read-only and gives the TEST XML overlay higher priority. It does not copy bundled artwork or include any new proprietary assets.

The optional sample fixture downloads only nine explicitly allowlisted files
from the existing rodata-dev:3339 route, writes a local SHA-256 pinned manifest
and content-addressed objects, then sets explicit offline mode. It enables
native loader/format checks without deploying or enabling any manifest server.
It is deliberately **not** a complete game install. No licensed artwork is
included in the repository; keep staged assets outside it. Fetch requires a
trusted dev service; hashes pin the observed bytes, not a signed release.
The bootstrap check uses the normal GameFileLoader/clientinfo parser without
window, GPU or login. It does not prove rendering, complete assets or gameplay.

Remote mode defaults to no proprietary local GRFs when game_archives.ron is
absent. Explicit local overlays retain priority and must exist. Malformed
archive settings and unavailable/invalid manifests fail with diagnostics;
missing defaults no longer trigger a native archive panic. Existing pinned
cache revisions are not silently updated. Use a fresh runtime/cache per revision.

For a complete dev install, stage an approved full raw asset snapshot with its
entire dependency set and use a separate cache (do not deploy production).
After sourcing its environment, copy the TEST profile overlay/settings into
that runtime. Run --check-bootstrap, then the normal binary in an actual
desktop/surface with supported Vulkan features. The current LXC has llvmpipe
but no DISPLAY/WAYLAND surface. A successful build/bootstrap is not graphical
verification. Do not change firewall/network settings to work around this.

Inventory/cart/storage transfers are isolated at the network layer. Until
separate container UI/state exists, completed non-player container snapshots
are ignored instead of being published as the ordinary inventory. This fixes
inventory corruption; it does not implement storage/cart gameplay.

## Lua bootstrap compatibility

The embedded VM is Lua 5.1. Server `.lub` files can be either bytecode (the dev
assets include 32-bit-size_t Lua 5.1 chunks) or plain source with legacy encoded
string literals. The normal loader now adapts recognized Lua 5.0/5.1 bytecode
in memory, using lunify2's byte-preserving converter. Lunify1's converter could
transcode raw strings during ABI conversion; do not reinstate the old low-byte
Unicode hack to compensate. Plain source is passed through unchanged except
for an optional UTF-8 BOM; UTF-16 is rejected with a named error.

Names/resource strings are decoded from the original Lua bytes: valid UTF-8 is
preserved exactly, otherwise strict Windows-949/EUC-KR decoding is attempted.
Invalid text is an error, not lossy normalization. Syntax, bytecode or table
conversion failures do not trigger rewriting, deletion or remounting of caches.
Legacy generated `lua_files.7z` overlays are no longer loaded; they are left
untouched on disk, and user-selected archive/translation precedence is retained.

With the isolated17-file bootstrap fixture, the actual normal executable now
passes all seven Lua library initializers and reaches the hard-coded initial
menu map `geffen`. That fixture does not include `data/geffen.rsw` or its full
dependency graph, so startup still stops there. Stage an approved complete menu
map dependency manifest before expecting a window; do not equate this progress
with rendering, audio playback, or a complete offline installation.

The exact staged-file probe found that `skillinfoz/skillinfolist.lub` is valid
Lua source, not corrupt bytecode. Its `SKILL_INFO_LIST[779].SkillName` is
non-UTF-8 but valid Windows-949/EUC-KR; the old Rust String conversion failed
before its encoding helper could run. All skill resource-name strings in that
fixture were valid UTF-8. The old unconditional archive-repair retry hid this
primary failure behind a later error. Synthetic tests cover this display-only
case as well as resource names and Unicode-preservation regressions.

A separate20-entry pinned fixture adds only Geffen RSW/GND/GAT via the existing
dev asset route. With it, normal startup passes Lua and the initial map-loader
call, then fails at window creation because neither DISPLAY nor WAYLAND_DISPLAY
nor WAYLAND_SOCKET is available. Missing dependent models/textures can fall
back, so passing the map-loader call is NOT complete-map/graphics validation.
Next graphical prerequisite: an existing interactive Vulkan-capable display
session for the TEST runtime (or separately authorized desktop provisioning),
plus complete approved map dependencies. No display-server/network setup or
production asset-service change was performed.
