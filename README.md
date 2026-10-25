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
| `iocc powmon mode <MODE> [--no-save]` | sets and saves the operating mode (`--no-save` keeps it only until the chip is reconfigured) |
| `iocc powmon calibrate --max <A> [--no-save]` | calibrates for a maximum expected current and saves it |
| `iocc powmon reset` | resets the chip |

`--json` prints the machine readable form used by the TUI (`status` as a table, `read` and `watch` as
metrics). `watch` prints one line per sample and keeps running until interrupted; `--interval-ms`
accepts values from 10 ms up (default 200).

## Saved settings

`mode` and `calibrate` write the new value to the `powmon.chip` section of `config.yaml` (only that
line: comments, indentation and the other sections are kept), so the next command — and the TUI
manifest — starts with it. Use `--no-save` for a temporary change:

```sh
iocc powmon calibrate --max 2      # chip calibrated for 2 A and maxcurrent: 2 saved
iocc powmon mode ShuntAndBusContinuous   # mode applied and saved
iocc powmon calibrate --max 5 --no-save  # this run only
```

The plugin receives the config path from iocc through `IOCC_CONFIG`; outside `iocc` it cannot read or
save the shared file.

## TUI

`iocc tui` lists powmon with a **Live readings** panel (streaming every 200 ms, with sparklines), two
graphs (**Voltage graph** and **Current graph**, line charts of the last 120 samples, one every 500 ms)
and a **Status** table (every 5 s), plus three actions (Set mode, Calibrate, Reset chip). It describes
itself through `--iocc-manifest`; see [`docs/tui-plugins.md`](../iocc/docs/tui-plugins.md) for the
protocol.

All four views and the *Set mode* action declare `"device": "shared"`, so they keep sampling while
other plugins use the bus. *Calibrate* and *Reset chip* are `exclusive`: they change or invalidate the
chip state, so the watches pause while they run and restart afterwards.

**Set mode** and **Calibrate** save their value in `config.yaml`. The manifest is generated from that
file, so the prompts open with the value in use — calibrate for `2` once, close the TUI and the next
`Calibrate` prompt already shows `2`. The TUI also asks for the manifest again after a successful
action, so even in the same session the next prompt is up to date.

### Graphs

**Voltage graph** and **Current graph** are separate panels fed by their own 500 ms stream, so both can
be on screen at the same time: select one with `[`/`]` and press `p`, do the same for the other, and the
*Watches* column keeps them stacked while you use other plugins. Each panel shows the live value,
min/max and a Braille line chart with a time axis over the last 120 samples (about 60 s).

Press `m` on a graph to switch its series: **Bus voltage**, **Current**, **Power** and **Shunt**. Use it
to get a power or shunt graph without another panel, or to keep the two pinned graphs on the series you
care about. Press `z` to zoom a panel to the full body; on a very short panel the chart falls back to a
sparkline. The graphs are visual only; pin **Live readings** as well if you want to record the numeric
series with `g`.

The graph panels need an iocc version that understands the `chart` view kind (1.2.0 or newer); older
TUI versions reject the manifest.

### Watching the current while another plugin runs

1. `iocc tui`, tab **3** (Plugins), select `powmon` and press `p` to pin **Live readings**.
2. Select another plugin and run a shared action: the powmon panel stays live in the *Watches* column.
3. `P` unpins everything.

Pinning is per panel: selecting `powmon` starts on **Live readings**, but `[`/`]` switches to
**Voltage graph**, **Current graph** or **Status** and `p` pins any of them. The selected watch can be
removed with `x`, all with `P`, and `PageUp`/`PageDown` scrolls the column when it does not fit. Press
`g` to record the pinned metrics (current included) to CSV.
[`examples/iocc-wave`](../iocc/examples/iocc-wave) shows the same flow without hardware.

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
only works for plugins that implement the protocol (powmon does).

The native commands have an explicit opt-in for the same broker:

```sh
iocc --via-tui scan
iocc --via-tui read -a 4A -r 02 -e 2
```

Without `--via-tui` they keep opening the device directly, and `--via-tui` is rejected for plugins
(use `IOCC_SOCKET` as shown above).

## Notes

- Current and shunt voltage are read as signed values.
- The current LSB used to scale readings is the one computed by the calibration, so
  `maxcurrent` in the config defines the resolution and the full scale.
- Communication errors are reported as `Not found on address` with exit code 1 instead of panicking.
- `read` and `watch` configure and calibrate the chip on start, so restart them after editing
  `config.yaml` by hand. Changing the values from the commands above or from the TUI already restarts
  the TUI panels and is saved for the next run.
- `watch` is the streaming form used by the TUI. It reads directly when `IOCC_SOCKET` is unset and
  through the TUI broker when the TUI sets it.

### Troubleshooting

| Symptom | Cause and fix |
|---|---|
| `USB error: Resource busy` | another process holds the MCP2221 (a running TUI, or another `iocc powmon`). Close it, or point `IOCC_SOCKET` at the running TUI as shown above. |
| `cannot connect to the iocc TUI broker at ...` | `IOCC_SOCKET` is set but stale. Unset it to read the device directly. |
| `device in exclusive use by a plugin` | an exclusive view/action is using the chip in the TUI; wait for it to finish. |
| The TUI panel stops with `command exited` | `watch` ended (the device was unplugged or a communication error occurred). Press `R` in the TUI to restart it. |
