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
| `iocc powmon voltage` / `amps` / `power` / `shunt` | a single reading |
| `iocc powmon mode <MODE> [--save]` | sets the operating mode (`--save` edits only the `mode:` line of config.yaml) |
| `iocc powmon calibrate --max <A>` | calibrates for a maximum expected current |
| `iocc powmon reset` | resets the chip |

`--json` prints the machine readable form used by the TUI (`status` as a table, `read` as metrics).

## TUI

`iocc tui` lists powmon with two panels (Readings, live at 1 s; Status at 5 s) and three actions
(Set mode, Calibrate, Reset chip). It describes itself through `--iocc-manifest`; see
[`docs/tui-plugins.md`](../iocc/docs/tui-plugins.md) in the iocc repository for the protocol.

## Notes

- Current and shunt voltage are read as signed values.
- The current LSB used to scale readings is the one computed by the calibration, so
  `maxcurrent` in the config defines the resolution and the full scale.
- Communication errors are reported as `Not found on address` with exit code 1 instead of panicking.
