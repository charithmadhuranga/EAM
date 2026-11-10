# EAM Test Device

A real ESP32 running ESPHome firmware, used for Phase 0+ integration tests.

## Firmware

`test-device.yaml` compiles a firmware with exactly the entities EAM's live
tests expect:

| Entity | Type | Why |
|---|---|---|
| Test Switch | Switch | Phase 0: switch command + state echo |
| Test Number | Number | Phase 0: number command + state echo |
| Test Light | Light | Phase 1: multi-field entity states |
| Uptime / WiFi Signal | Sensors | Phase 2: historical data recording |
| Status | Binary Sensor | availability/state handling |
| ESPHome Version | Text Sensor | non-numeric state values |

Credentials live in `secrets.yaml` (gitignored — **never commit**).

## Setup

```sh
# 1. install the CLI once (needs Python 3.10+)
python3 -m venv /tmp/opencode/esphome-venv
/tmp/opencode/esphome-venv/bin/pip install esphome

# 2. validate
/tmp/opencode/esphome-venv/bin/esphome config tests/device/test-device.yaml

# 3. flash over serial (find the port with: ls /dev/ttyUSB* /dev/ttyACM*)
/tmp/opencode/esphome-venv/bin/esphome run tests/device/test-device.yaml --device /dev/ttyUSB0

# 4. confirm it is on the network (should answer with eam-test-device)
avahi-browse -rtp _esphomelib._tcp
```

Or via the Makefile: `make flash-test-device SERIAL_PORT=/dev/ttyUSB0`.

The device falls back to the hotspot `EAM-Test-Fallback` if it cannot join the
configured Wi-Fi, so it stays reachable for the API tests.

## Running the live tests

```sh
EAM_LIVE_DEVICE=10.60.109.61:6053 make test-live
```

`crates/eam-esphome-api/tests/live.rs` skips (prints `SKIP:`) when
`EAM_LIVE_DEVICE` is unset, so CI and `cargo test --workspace` are unaffected.

## Current device state

- **Host:** `10.60.109.61:6053` (DHCP; re-discover with `avahi-browse`)
- **mDNS:** `eam-test-device.local`
- **Board:** ESP32 dev board (`esp32dev`), Arduino framework
- **Firmware:** ESPHome 2026.10.2, no API encryption
- **Entities:** 7 (switch, number, light, 2 sensors, binary sensor, text sensor)

Wi-Fi credentials and any future OTA/encryption keys belong in `secrets.yaml`
only. AGENTS.md rule 7: never store plaintext secrets in the repository.
