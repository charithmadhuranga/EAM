# ADR-009: ESPHome Native API Client Strategy

- **Status:** Accepted
- **Date:** 2026-11-10
- **Phase:** 0 (Technical Validation Spike)
- **Deciders:** EAM maintainers
- **Supersedes:** none
- **Related:** AGENTS.md, `docs/architecture/PLAN.md` (Phase 0), spec §"Protocol Spike"

## Context

Phase 0 requires validating the highest-risk integration — the ESPHome Native
API — before committing the architecture. The app must connect to real
ESP32/ESP8266 devices, discover entities, stream live states, and control
entities (switch, number) with reconnect recovery.

Two decisions had to be made: how to speak the protocol, and how to keep the
protocol contained.

## Decision

### 1. Use the official protocol definitions as source of truth

`crates/eam-esphome-api/proto/` contains `api.proto` and `api_options.proto`
from the ESPHome repository (tag `2026.9.1`, the device firmware line).
They are compiled with `prost-build` via `examples/regen_proto.rs`, and the
generated `src/api_proto.rs` is **checked in** so normal builds — including CI —
do not need `protoc` installed.

`src/message.rs` (the message-ID table, 149 types) is generated from the same
file by `examples/regen_proto_ids.rs`, so protocol constants are never
hand-maintained.

### 2. Keep all protocol knowledge inside the ESPHome adapter

`eam-esphome-api` is the only crate that knows about the wire format, message
IDs, or protobuf types. It exposes `ApiClient`, `DeviceInfo`, `StateUpdate`;
everything crossing the boundary is converted into `eam-domain` types
(`Device`, `Entity`, `EntityValue`). `eam-domain` stays protocol-free.

### 3. Plaintext first; Noise encryption deferred (DEV-004)

The test device was flashed without `api.encryption`, so this phase validated
the plaintext handshake. Password authentication was **removed** from the
protocol in ESPHome 2026.1.0 (message IDs 3/4 are reserved and rejected), so
encryption is a required follow-up, tracked as spec item **DEV-004**.

### 4. Framing follows the documented layout exactly

`[0x00][varint payload_len][varint message_type][protobuf payload]`.

The length varint covers the payload only — *not* the type. This was verified
against a real capture (device replied `00 20 02 ...` for a 32-byte
`HelloResponse`); an earlier, incorrect implementation that included the type
byte in the length desynchronised the stream and was caught by unit tests.

### 5. Commands are validated locally against the listed entities

`ApiClient` rejects commands for unknown entity keys (`Error::UnknownKey`)
before sending, and `entity_for_key` maps protocol keys back to domain
`EntityId`s. Entity keys are 32-bit values, not small integers — they are
device-computed hashes (e.g. `1040761112`) and must never be assumed sequential.

### 6. Reconnect recovery is a client-contract, not an internal retry

The client owns one connection. Callers drop and re-create the client on
recoverable errors (`Error::is_recoverable` covers connection resets,
handshake timeouts, protocol violations). Auto-reconnect with backoff belongs
to the device manager/runtime (Phase 1), not the transport.

## Consequences

- Protocol upgrades require re-running the two regen examples, not hand edits.
- CI has no `protoc` dependency; generating does.
- Timeouts are per-call (`frame_timeout`), bounded frames are capped at 64 MiB.
- Real-device tests are opt-in via `EAM_LIVE_DEVICE` and always runnable.

## Alternatives considered

| Option | Why rejected |
| --- | --- |
| Vendoring the ESPHome Python `aioesphomeapi` client | Async Python inside a Rust core; duplicates state and blocks our runtime. |
| Hand-written prost structs without the proto file | Loses the official definition as source of truth (spec §1383: "Use official ESPHome API protocol definitions"). |
| Protobuf codegen in `build.rs` | Every build and CI job would need protoc installed. |
| Auto-reconnect inside `ApiClient` | Hides session state (subscriptions must be re-established) from the caller. |

## Validation (real device)

Results against `eam-test-device` (ESP32, ESPHome 2026.10.2, plaintext API):

| Check | Result |
| --- | --- |
| Hello handshake | API 1.19 negotiated, server name returned |
| Device info | name/MAC/version/board/model all populated |
| Entity listing | 7 entities: 1 switch, 1 number, 1 light, 2 sensors, 1 binary sensor, 1 text sensor |
| Live states | burst of states after `subscribe_states`, all keys present in the listing |
| Switch command | OFF→ON→OFF round-trip, device echoes each change |
| Number command | value 77.0 reported back by the device |
| Reconnect | abrupt close then reconnect: identical 7-entity list, ping OK |
| Unknown key | `Error::UnknownKey` raised locally before any I/O |
| Toolchain | `esphome config` + serial flash to `/dev/ttyUSB0` succeeded |

Live tests: `crates/eam-esphome-api/tests/live.rs`, all passing.
