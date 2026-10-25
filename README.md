# iocc-powmon

`iocc` plugin for the **INA226** power monitor (voltage, current, power and shunt voltage) on the
MCP2221 bus. It is not standalone: it only runs through `iocc powmon ...`, which sets `IOCC_HOST`
and `IOCC_CONFIG`.

Settings live in the `powmon:` section of the shared `config.yaml` (see `config.yaml`):

```yaml
powmon:
  address: 0x4A
  verbose: false
  i2c_speed_hz: 400000
  reset_on_open: false
  timeout: 10
  chip:
    mode: BusVoltageContinuous
    vbusct: _1_1_ms
    vscht: _1_1_ms
    average: _16
    maxcurrent: 5.0
```

Missing keys or a missing section fall back to these defaults.

## Install

Install `iocc` first. The script puts the plugin next to the `iocc` found in `PATH` (or in
`~/.local/bin` when iocc is not installed yet); use `--prefix DIR` to match a custom folder.

```sh
./install.sh                 # builds and installs next to iocc (or ~/.local/bin)
./install.sh --prefix DIR    # explicit folder
./install.sh --uninstall
```

## Commands

| Command | Output |
|---|---|
| `iocc powmon status [--json]` | verifies manufacturer/die and decodes the configuration register |
| `iocc powmon read [--json]` | bus voltage, current, power and shunt voltage |
| `iocc powmon watch [--json] [--interval-ms 200]` | keeps printing readings until interrupted |
| `iocc powmon voltage` / `amps` / `power` / `shunt` | a single reading |
| `iocc powmon mode <MODE> [--save]` | sets the operating mode (`--save` edits only the `mode:` line of config.yaml) |
| `iocc powmon calibrate --max <A>` | calibrates for a maximum expected current |
| `iocc powmon reset` | resets the chip |

`--json` prints the machine readable form used by the TUI (`status` as a table, `read` and `watch` as
metrics). `watch` prints one line per sample and keeps running until interrupted; `--interval-ms`
accepts values from 10 ms up (default 200).

## TUI

`iocc tui` lists powmon with a **Live readings** panel (streaming every 200 ms, with sparklines) and a
**Status** table (every 5 s), plus three actions (Set mode, Calibrate, Reset chip). It describes itself
through `--iocc-manifest`; see [`docs/tui-plugins.md`](../iocc/docs/tui-plugins.md) for the protocol.

Both views and the *Set mode* action declare `"device": "shared"`, so they keep sampling while other
plugins use the bus. *Calibrate* and *Reset chip* are `exclusive`: they change or invalidate the chip
state, so the watches pause while they run and restart afterwards.

### Watching the current while another plugin runs

1. `iocc tui`, tab **3** (Plugins), select `powmon` and press `p` to pin **Live readings**.
2. Select another plugin and run a shared action: the powmon panel stays live in the *Watches* column.
3. `P` unpins everything.

Pinning keeps one panel per plugin (powmon's streaming *Live readings*), so pin another plugin's panel
to watch two signals at once. [`examples/iocc-wave`](../iocc/examples/iocc-wave) shows the same flow
without hardware.

### Command line and the broker

Command line use is unchanged: iocc does not set `IOCC_SOCKET` for `iocc powmon ...`, so `status`,
`read`, `mode`, `calibrate`, `reset` and `watch` keep opening the MCP2221 directly, with the same
output, errors and exit codes as before. `watch` is the only addition.

If you want the command line to read through an already running TUI, export the socket yourself:

```sh
IOCC_SOCKET=/run/user/1000/iocc.sock iocc powmon read --json
IOCC_SOCKET=/run/user/1000/iocc.sock iocc powmon watch --json
```

The TUI socket is `$XDG_RUNTIME_DIR/iocc.sock` unless it was started with another `IOCC_SOCKET`. This
only works for plugins that implement the protocol (powmon does); the native `iocc scan`/`read`/`write`
commands always open the device directly.

## Notes

- Current and shunt voltage are read as signed values.
- The current LSB used to scale readings is the one computed by the calibration, so
  `maxcurrent` in the config defines the resolution and the full scale.
- Communication errors are reported as `Not found on address` with exit code 1 instead of panicking.
- `read` and `watch` configure and calibrate the chip on start, so restart them after changing
  `maxcurrent` or `mode`, or after resetting the chip.
- `watch` is the streaming form used by the TUI. It reads directly when `IOCC_SOCKET` is unset and
  through the TUI broker when the TUI sets it.

### Troubleshooting

| Symptom | Cause and fix |
|---|---|
| `USB error: Resource busy` | another process holds the MCP2221 (a running TUI, or another `iocc powmon`). Close it, or point `IOCC_SOCKET` at the running TUI as shown above. |
| `cannot connect to the iocc TUI broker at ...` | `IOCC_SOCKET` is set but stale. Unset it to read the device directly. |
| `device in exclusive use by a plugin` | an exclusive view/action is using the chip in the TUI; wait for it to finish. |
| The TUI panel stops with `command exited` | `watch` ended (the device was unplugged or a communication error occurred). Press `R` in the TUI to restart it. |
