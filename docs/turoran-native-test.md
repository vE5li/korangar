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
