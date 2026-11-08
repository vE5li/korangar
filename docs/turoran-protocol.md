# Turoran Hercules protocol contract (20190530 RE)

Status: **implementation reference, not a claim of client parity**. Keep the
existing 20220406 backend working. No production endpoints, accounts, captures,
credentials, or Gravity assets belong in tests or this repository.

## Source of truth and reproducibility

This audit pins these source revisions rather than a moving branch:

- Hercules: [`3846d1b4d3c1d8bf5e3a739f43c2206cc27bd0ed`](https://github.com/Turor/hercules-prere-plus/tree/3846d1b4d3c1d8bf5e3a739f43c2206cc27bd0ed).
- roBrowser: [`3239980f4a69a3146d1c96d44dcce7104ebdc274`](https://github.com/Turor/roBrowserLegacy/tree/3239980f4a69a3146d1c96d44dcce7104ebdc274).
- Korangar starting point: `639eaabfeb6901b68697739ea84f7375e4c26783`.

Hercules is configured with `--enable-packetver=20190530 --enable-packetver-re`
and Renewal gameplay. Packet version, packet family, gameplay rules, and custom
extensions are independent settings: a generic 2019 client is not sufficient.

Authoritative server files at the pinned revision:

| Area | Source |
| --- | --- |
| Map requests, feature gates | `src/map/packets.h` |
| Shuffled request opcodes/field offsets | `src/map/packets_shuffle_re.h` |
| Map response layouts | `src/map/packets_struct.h`, `src/map/clif.c` |
| Packet lengths | `src/common/packets_len.h`, `src/common/packets/packets*_len_re.h` |
| Login | `src/login/lclif.c`, `src/login/packets_ca_struct.h`, `src/login/packets_ac_struct.h` |
| Character selection | `src/char/char.c`, `src/char/packets_hc_struct.h` |
| Custom systems | `src/plugins/turoran_stable.c`, `turoran_eden.c`, `turoran_warpra.c`, `turoran_mvp_minimap.c` |

Cross-check each against roBrowser `src/Network/PacketStructure.js`,
`PacketRegister.js`, `PacketVersions.js`, `PacketVerManager.js`, and the selected
packet-length table. A registered browser decoder alone does **not** establish
that this server build enables a feature. Do not copy GPL server implementation
code into the MIT client; document wire facts and implement independently.

## Transport, framing, and safety

- TCP to login, character, and map services; no UDP gateway in the parity milestone.
- All ordinary RO numeric fields are little-endian, but confirm address/port
  conversion per message; do not apply host-native struct layout.
- A two-byte opcode starts every ordinary packet. Variable packets then carry a
  two-byte **total** length including opcode and length. Fixed packets do not.
- TCP reads can contain fragments or several packets. Buffer until a complete
  packet is available; reject lengths below the packet's minimum and enforce a
  bounded receive buffer. Never guess a boundary after an unknown opcode.
- Character/map connection setup can include a raw account-ID prefix. Handle it
  in the connection state machine, not as an ordinary two-byte-opcode message.
- Fixed strings are byte fields with a specified encoding and maximum width,
  not Rust UTF-8 byte lengths by default. Confirm Korean text conversion.
- Never log password packets, auth codes, session IDs, or account credentials.
  Synthetic fixtures must use invented IDs; captures must be sanitized.
- Negotiation is explicit: a 20190530 RE profile must not fall back silently to
  20220406 structs. Keep response decoding and request encoding version-aware.

## Connection-state contract

1. **Login:** send the supported login request (`0x0064` is the existing
   Korangar path). Decode success/failure, preserve both login IDs, sex and
   account ID, and select an advertised character endpoint. roBrowser registers
   `0x0ac4` as `ACCEPT_LOGIN3`; older login success variants also exist. The
   actual server response for the test build must be fixed by a synthetic
   fixture derived from its gated server layout before enabling the profile.
2. **Character:** connect using the login session, consume the account prefix,
   negotiate/list slots and characters, handle PIN/failure/status packets, then
   select a slot and extract the character ID and map endpoint. Do not reuse
   a 2022 character-record layout without checking every gated field and width.
3. **Map:** send the 19-byte `0x0436` request below; consume connection prefix and
   enter response, load the map/assets, then send load-end acknowledgement.
   Register every packet the server emits on map entry, including plugin packets,
   before attempting an interactive session.
4. **Gameplay:** ticks, walk/action/skill requests, inventory and equipment,
   NPC conversation, warp and reconnect must all work before calling the first
   milestone playable. Disconnect/reconnect must discard the previous parser
   state and pending world updates.

Login/character **field-by-field fixture validation remains a release gate**;
this initial audit is not a complete login/character codec implementation.

## Verified map request subset

These opcodes/offsets are selected by the `20190530` arm of
`packets_shuffle_re.h` (lines 9672 and 9716-9744), and match the browser builders.
Sizes include the opcode. `u8/u16/u32` denote unsigned little-endian wire values.

| Opcode | Bytes | Payload (offsets from packet start) |
| --- | ---: | --- |
| `0x0436` | 19 | account:u32 @2, character:u32 @6, login_id1:u32 @10, tick:u32 @14, sex:u8 @18 |
| `0x007d` | 2 | map load-end acknowledgement, no payload |
| `0x035f` | 5 | packed world position (3 bytes) @2 |
| `0x0360` | 6 | tick:u32 @2 |
| `0x0361` | 5 | head direction:u16 @2, body direction:u8 @4 |
| `0x0437` | 7 | target entity:u32 @2, action:u8 @6 |
| `0x0438` | 10 | skill level:u16 @2, skill ID:u16 @4, target entity:u32 @6 |
| `0x0366` | 10 | skill level:u16 @2, skill ID:u16 @4, x:u16 @6, y:u16 @8 |
| `0x0362` | 6 | ground item entity:u32 @2 |
| `0x0363` | 6 | inventory index:u16 @2, quantity:u16 @4 |
| `0x0364` | 8 | inventory index:u16 @2, quantity:u32 @4 (to storage) |
| `0x0365` | 8 | storage index:u16 @2, quantity:u32 @4 (from storage) |

**First confirmed incompatibility:** existing Korangar `MapServerLoginPacket`
adds four bytes for its 2022 profile. The 2019 request must not include that
field. Browser `CZ.ENTER2` adds it only from `20210630`, and the Hercules
shuffle arm places sex at offset 18 for our version. Changing only the enum or
opcode would send an invalid session handshake.

## Turoran extension and feature inventory

All sizes below are total wire sizes; `variable` requires bounded framing and
per-entry count/size validation. Implement UI and server feature availability
separately. No client-side upgrade/rate/cost authority is permitted.

### Enchant UI (explicitly backported to RE 20190530)

`packets.h` lines 2071-2077 and `packets_struct.h` lines 5832-5884 establish:

| Opcode | Direction | Bytes | Payload |
| --- | --- | ---: | --- |
| `0x0b9a` | S→C | 11 | UI_OPEN_V3; browser UI dispatcher, validate server gate/layout |
| `0x0b9b` | C→S | 12 | enchant group:i64, inventory index:i16 |
| `0x0b9c` | C→S | 16 | enchant group:i64, inventory index:i16, item ID:i32 |
| `0x0b9d` | C→S | 14 | enchant group:i64, inventory index:i16, slot:i16 |
| `0x0b9e` | C→S | 12 | enchant group:i64, inventory index:i16 |
| `0x0b9f` | S→C | 10 | message ID:i32, item ID:i32 |
| `0x0ba0` | C→S | 2 | close enchant UI |

Preserve all 64 bits of enchant group IDs; don't truncate via a 32-bit UI ID.

### Custom plugin packets (not part of stock 20190530)

These are confirmed by the server plugins' `addPacket` registrations / senders
and browser builders/decoders around lines 16025-16272.

| Opcode | Direction | Bytes | Meaning |
| --- | --- | ---: | --- |
| `0x0ef0` | C→S | 2 | stable open |
| `0x0ef1` | C→S | 6 | stable homunculus request |
| `0x0ef2` | C→S | 8 | stable pet request |
| `0x0ef3` | C→S | 8 | Eden request |
| `0x0ef4` | C→S | 8 | Warpra request |
| `0x0ef5` | S→C | variable | alive MVP minimap positions |
| `0x0ef6` | S→C | variable | stable owned list |
| `0x0ef8` | S→C | variable | stable homunculus list |
| `0x0ef9` | S→C | variable | stable pet list |
| `0x0efa` | S→C | 8 | stable result |
| `0x0efb` | S→C | variable | Eden quest list |
| `0x0efc` | S→C | 8 | Eden result |
| `0x0efd` | S→C | variable | Eden shop |
| `0x0efe` | S→C | variable | Warpra destination list |
| `0x0eff` | S→C | 8 | Warpra result |

Do not treat the unused `0x0ef7` slot as a packet. The MVP plugin explicitly
warns that failing to register `0x0ef5` as variable length desynchronizes map entry.

### Existing systems requiring parity

- **Memorial dungeon:** browser registers `0x02cb` (65), `0x02cc` (4), `0x02cd`
  (71), `0x02ce` (10); validate command and reconnect/timer behavior too.
- **Elementals:** `0x081d` EL_INIT and `0x081e` EL_PAR_CHANGE update state and
  the spirit's HP/SP bars. Do not invent a separate official elemental window.
- **Equipment switch:** requests `0x0a97`, `0x0a99`, `0x0a9c`, `0x0ace`;
  responses `0x0a98`, `0x0a9a`, `0x0a9b`, `0x0a9d`. Inventory-index translation
  and 32-bit position masks must round-trip; authoritative state is on server.
- **Stylist:** browser registers `0x0a46`, `0x0a47`, `0x0afc`; select the gated
  request variant and parse the shared UI-open dispatcher before drawing the UI.
- **Battlegrounds, trade/storage, vending, quests, pets/homunculi, party/guild,
  mail, refine, synthesis/upgrade, barter and enchant/reward NPCs:** inventory
  of required behavior, not an assertion that upstream Korangar supports them.
- **Grade enchant is not automatically enabled:** `packets.h` lines 2056-2060
  gate `0x0b59/0x0b5b/0x0b5c` behind RE >= 20191016. The browser containing
  decoders is not proof of a 20190530 server backport. Likewise trait-stat
  `0x0b24` is behind a newer version gate. Confirm an explicit server extension
  before emitting either. Do not turn on grade-above-A gameplay as client work.

## Persistent remote-asset contract (next independent milestone)

The client must use local archives/cache first, then the configured remoteclient
endpoint. Development uses the rodata-dev test service on port 3339; public
production assets are under `https://ro.turoran.com/client/`. Neither is a login
or gameplay transport. Production must never silently become a test fallback.

- Normalize RO virtual paths, reject traversal/absolute paths and platform drive
  prefixes, resolve archive precedence exactly as the current remoteclient does.
- A versioned manifest must enumerate only allowed virtual game assets with
  canonical path, decoded byte length and SHA-256. Never expose filesystem roots,
  configuration, database, keys, logs, or arbitrary directory listing endpoints.
- Manifest and fetched contents must describe the same snapshot. Pin a revision
  and verify size/hash before publishing a cache entry. Hashes detect corruption;
  authentication still depends on the trusted HTTPS origin/manifest.
- Stream downloads into exclusive temporary files; bound response sizes, verify
  bytes, fsync and atomically rename on the same filesystem. Interrupted downloads
  must not replace a valid entry. Deduplicate concurrent requests for one asset.
- Keep every verified file permanently. No LRU eviction/cleanup of downloaded
  assets. Cache metadata records manifest revision and verified file digest.
- A missing manifest must not mean "complete". Mark a snapshot complete only
  after every listed asset is verified locally. Check completeness on startup
  without making an HTTP request; explicit update checks are separate.
- Once complete, the selected snapshot works entirely offline. Local corruption
  makes completeness false; do not report success from filenames alone.
- Add the manifest server in a **separate, undeployed** remoteclient PR. No
  Gravity art is committed or bundled; read the player's installed game data.

## Acceptance gates and current status

- [x] Pin server/browser revisions and identify 19-byte map-login incompatibility.
- [x] Identify enchant and custom plugin opcode families from both endpoints.
- [ ] Implement independent 20190530 request/response codecs and synthetic tests.
- [ ] Add selectable backend only when all three connection phases are safe.
- [ ] Validate character/item widths, all map-enter packets, and parser boundaries.
- [ ] Login → character selection → map load on **hercules-test only**, with a
  dedicated account, then walk/attack/one skill/inventory/NPC conversation/warp.
- [ ] Test all custom systems, failure paths, reconnect and baseline 20220406.
- [ ] Persistent cache, integrity/interruption/path tests and offline completion.
- [ ] Allowlist manifest server PR; no production deployment or automatic merge.

Do not claim parity until the unchecked gates have recorded evidence. Preserve
upstream MIT attribution and keep credentials and game artwork out of all PRs.
