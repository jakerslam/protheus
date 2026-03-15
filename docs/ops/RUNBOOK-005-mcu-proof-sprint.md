# RUNBOOK-005: Tiny-Max MCU Proof Sprint

## Purpose

Produce real hardware proof for Tiny-max on `ESP32` and `RP2040`:

- flash session logs,
- runtime status output,
- screenshots/photos,
- reproducible command transcript.

This runbook is linked to `P0-MCU-PROOF-001` and unblock action `HMAN-092`.

## Preconditions

1. Physical hardware connected over USB:
   - one `ESP32` dev board,
   - one `RP2040` board (for example Pico/Pico W).
2. Tooling installed:
   - `openocd`
   - `picotool`
   - `esptool` (or `esptool.py`)
   - `screen` (or equivalent serial monitor)
3. Preflight passes:

```bash
node tests/tooling/scripts/ci/mcu_proof_preflight.mjs
cat local/workspace/reports/MCU_PROOF_PREFLIGHT.md
```

## Evidence Targets

Store evidence under:

- `docs/client/reports/hardware/esp32_tiny_max_status_<date>.png`
- `docs/client/reports/hardware/rp2040_tiny_max_status_<date>.png`
- `state/ops/evidence/mcu_flash_session_<date>.md`

Required transcript sections in `mcu_flash_session_<date>.md`:

1. tool versions,
2. connected serial ports,
3. flash command used per board,
4. boot/runtime serial output,
5. command proving runtime status.

## Step A: Tool + Device Discovery

```bash
openocd --version | head -n 2
picotool version
esptool version || esptool.py version
ls /dev/tty.usb* /dev/cu.usb* 2>/dev/null
```

## Step B: ESP32 Flash + Runtime Capture

```bash
# Example; adjust chip/offsets to your board + image layout
esptool --port /dev/cu.usbserial-XXXX chip_id
esptool --port /dev/cu.usbserial-XXXX write_flash 0x10000 build/esp32/protheusd.bin
screen /dev/cu.usbserial-XXXX 115200
```

Capture:

- boot output showing Tiny-max startup,
- status command output (or equivalent runtime banner),
- screenshot/photo.

## Step C: RP2040 Flash + Runtime Capture

```bash
picotool info -a
# board in BOOTSEL mode; copy UF2 or flash via openocd workflow
cp build/rp2040/protheusd.uf2 /Volumes/RPI-RP2/
screen /dev/cu.usbmodemXXXX 115200
```

Capture:

- boot output showing Tiny-max startup,
- status command output (or equivalent runtime banner),
- screenshot/photo.

## Step D: Record Final Evidence Bundle

1. Write `state/ops/evidence/mcu_flash_session_<date>.md`.
2. Place screenshots in `docs/client/reports/hardware/`.
3. Update README hardware proof section from `blocked` to `verified`.

## Current Known Blockers

- Without connected USB boards, this runbook cannot be completed.
- Current host daemon artifact (`x86_64-unknown-linux-musl`) is not directly flashable to MCU; MCU-specific build artifact generation must be provided by the embedded lane before flashing.

