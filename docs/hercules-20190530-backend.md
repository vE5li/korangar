# Historical 20190530 RE backend (not the deployed target)

This is an explicit, selectable profile alongside the existing 20220406 default,
not a claim of complete official-client or Turoran custom-UI parity.

Select `<packet_version>20190530</packet_version>` in the server's clientinfo entry,
or `SupportedPacketVersion::_20190530` in the networking API. No fallback/default
version is changed. This profile requires Turoran's wide 175-byte character
records and three forge-option extension, not an unmodified upstream 2019 server.

## Wire changes

* Login advertises version 20190530; shared login/character authentication codecs
  retain their matching layouts. Authentication/session payloads are excluded
  from packet inspector callbacks, including parser failures.
* Character pages use 0x099d, character creation 0x006d, slot responses 0x08d5.
* Map authentication is 0x0436 / 19 bytes, with sex at offset 18.
* Walk requests use 0x035f, ground skills 0x0366 / 10 bytes, public chat 0x00f3.
* Equipment lists use 0x0b0a with 82-byte records; pickup 0x0a37 is 84 bytes.
  Refinement precedes cards and three forge options follow five random options.
  Forge metadata is preserved in `InventoryItemDetails`, not thrown away.
* Matching actor, attack, skill, inventory-boundary, NPC, and map-change codecs
  feed the existing network event API. Incompatible newer response signatures
  are rejected instead of being silently treated as 2019.
* Effective opcode-length facts are pinned to source revision
  `92c6a4d722bf002c5adae0ec03dfdd9a000b4163`, evaluated as 20190530 RE,
  TURORAN_WIDE_HPSP, MAX_FORGE_OPTIONS=3. These are wire facts, not copied server
  implementation. Custom plugin framing is explicit.
* TCP input is incrementally buffered (128 KiB limit), handles fragmented raw
  account prefixes and frames larger than 8192 bytes, and validates exact decoder
  consumption. Unknown-length or malformed input fails closed. Known-length
  packets without a semantic handler are skipped at their exact boundary and
  reported by opcode only.

## Local evidence

On korangar-dev LXC 119, unit tests cover 2019 handshake bytes, unchanged 2022
handshake bytes, wide HP offsets, equipment and pickup forge offsets, movement/
chat/ground-skill headers, malformed framing, authentication callback exclusion,
and an actual loopback TCP stream with a fragmented account prefix plus a 20 KB
plugin frame followed by a decoded gameplay event. The full graphical client
compiles without requiring any Gravity assets for these checks.

Commands:

```
cargo test -p ragnarok-packets -p korangar-networking --lib
cargo check -p korangar-networking --all-features
cargo check -p korangar --bin korangar
cargo run -p korangar-networking --example hercules_test -- /private/account.json
```

The headless example accepts only isolated test addresses 192.168.11.110 ports
6910/6131/5131 and a dedicated `korangar_m1_` account. It never logs credentials,
login session IDs, or packet payloads. It does not require artwork.

## Resolved target-identification issue — November 8, 2026 (CST)

The existing `/opt/hercules-test` checkout defaults to 20190530 in its source,
but its generated `config.status` says `--enable-packetver=20250716` and supplies
`-DPACKETVER=20250716`. Its running test services therefore do not match the
intended test profile. A guarded dedicated-account probe successfully completed
login and character authentication, but an explicit character-page request
received **0x0b72**, not 2019's **0x099d**. The new profile correctly refuses it.

No test service was rebuilt/restarted and no live service was changed. No test
character was created. The subsequent LIVE audit confirmed the same 20250716 MAIN profile.
The client was retargeted instead of changing either server build. See
`turoran-protocol.md` for live-target evidence and gameplay results.

## Remaining gates (not complete)

Actual test-server character creation/list/select, map acceptance, walking,
actor updates, attack, one successful skill, inventory, NPC dialogue, and
warp/reconnect are NOT yet validated against a matching server. The headless
example currently exercises only the initial subset; attack and warp/reconnect
coverage still need implementing. Known framing of custom plugin packets is not
their custom UI implementation. PIN-authentication UI and full Turoran feature
parity remain separate work. Do not merge this candidate as a completed gameplay
milestone merely because compilation and synthetic fixtures pass.
