#!/usr/bin/env python3
"""Stage a native TEST-only bootstrap and optional bounded dev asset samples.
This is NOT a complete game install or a production manifest deployment.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import urllib.request

SAMPLES = ["data/izlude.rsw", "data/izlude.gnd", "data/izlude.gat",
           "data/prontera.rsw", "data/prontera.gnd", "data/prontera.gat",
           "data/sprite/npc/4_f_kafra1.spr", "data/sprite/npc/4_f_kafra1.act",
           "data/texture/effect/angel.str"]
class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", required=True, type=Path, help="New private runtime directory outside the checkout")
    parser.add_argument("--sample-assets", action="store_true", help="Fetch nine allowlisted dev samples, pin hashes, then use offline cache only")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    runtime = args.runtime.resolve()
    if runtime == repo or repo in runtime.parents:
        parser.error("runtime must be outside the checkout")
    marker = runtime / ".korangar-test-profile"
    if runtime.exists() and any(runtime.iterdir()):
        parser.error("use a new empty runtime directory; existing files will not be overwritten")
    runtime.mkdir(parents=True, exist_ok=True, mode=0o700)
    marker.write_text("TEST bootstrap/sample fixture; not a complete install\n")
    overlay = runtime / "archive/data"
    overlay.mkdir(parents=True)
    xml = (repo / "profiles/turoran-test/clientinfo.xml").read_bytes()
    (overlay / "sclientinfo.xml").write_bytes(xml)
    (runtime / "client").mkdir()
    bundled = str(repo / "korangar/archive") + "/"
    # Keep bundled, already-licensed UI/font assets, with the TEST overlay last
    # (GameFileLoader gives the last listed archive highest priority).
    (runtime / "client/game_archives.ron").write_text("(archives: [" + json.dumps(bundled) + ', "archive/"])\n')
    env = ["# Source this file, then cd to its runtime directory. No credentials are stored here."]
    if args.sample_assets:
        cache = runtime / "sample-cache"
        objects = cache / "objects"
        objects.mkdir(parents=True)
        files = []
        opener = urllib.request.build_opener(NoRedirect())
        for path in sorted(SAMPLES):
            try:
                with opener.open("http://rodata-dev:3339/" + path, timeout=30) as response:
                    data = response.read(50 * 1024 * 1024 + 1)
                if len(data) > 50 * 1024 * 1024:
                    raise ValueError("sample size limit")
            except Exception:
                raise SystemExit("Dev sample fetch failed for " + path + "; no credentials or redirects are supported") from None
            digest = hashlib.sha256(data).hexdigest()
            (objects / digest).write_bytes(data)
            files.append(dict(path=path, size=len(data), sha256=digest))
        compact = json.dumps(files, separators=(",", ":")).encode()
        manifest = dict(format_version=1, revision=hashlib.sha256(compact).hexdigest(), files=files)
        (cache / "manifest.json").write_text(json.dumps(manifest, separators=(",", ":")))
        env += ["export KORANGAR_REMOTE_BASE_URL=http://rodata-dev:3339/",
                "export KORANGAR_REMOTE_ALLOW_HTTP=1", "export KORANGAR_REMOTE_OFFLINE=1",
                "export KORANGAR_REMOTE_CACHE=" + shlex.quote(str(cache))]
    env += ["cd " + shlex.quote(str(runtime))]
    (runtime / "run-env.sh").write_text("\n".join(env) + "\n")
    print("Staged TEST-only native profile (20250716, isolated login6910).")
    print("Samples staged: " + str(len(SAMPLES) if args.sample_assets else 0) + "; NOT full offline asset completeness.")
    print("Source run-env.sh in the runtime and run korangar --check-bootstrap first.")

if __name__ == "__main__":
    main()
