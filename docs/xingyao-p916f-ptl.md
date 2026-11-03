# Xingyao 14 P916F-PTL

This contract is scoped to the inspected MECHREVO `XINGYAO Series-P916F-PTL`,
Intel Core Ultra X7 358H, BIOS 1.00 dated 2026-03-14. STX and LNL protocols
have not been validated on this machine.

Inspected DSDT SHA-256:
`a6e21629835d72c9c8c7fbdfec31ae7ee9dcb99425a6044f0747590d2730392d`.

The exact PTL contract is selected before the older `P916F*` contract. Modes and key switches use
`root\wmi:OemWMIMethod.OemWMIfun`, with a 64-byte input and 256-byte output.
Read responses require byte 0 to be zero. Performance-mode writes return the
EC command echo in byte 0: `0x94` / `0x95` / `0x96` (148 / 149 / 150).
A successful WMI transport call alone is insufficient.

Manifest selection checks the manufacturer and declared BIOS range as well as
the board/product identity. An exact PTL identity with an unvalidated BIOS gets
a monitoring-only contract with no write channels or actions. It cannot fall
through to the generic P916F contract or an installed Tongfang driver.

| Operation | Request | Response | Evidence |
| --- | --- | --- | --- |
| Interface version | `01 01` | little-endian DWORD at byte 1 | Local DSDT `GVER`, live value `0x00020004` |
| Fan selector 0 | `02 01 00` | little-endian WORD at byte 1 | Local DSDT `GFNS`, live query |
| Fan selector 1 | `02 01 01` | little-endian WORD at byte 1 | Local DSDT `GFNS`, live query |
| Current performance mode | `03 09` | byte 1: 1 / 2 / 3 | Local DSDT `GPFM`, live query |
| Performance mode | `03 0A mode` | byte 0: `94` / `95` / `96` (hex) | Local DSDT `SPFM`: office / balance / turbo; live writes and readback |
| Fn lock | `03 04 enabled` | status byte | Local DSDT `SFLS` |
| Windows-key lock | `03 02 enabled` | status byte | Local DSDT `SWLS` |

Fan selectors require separate calls. The second fan is not a second WORD in
the first response. Zero is a valid stopped-fan reading and clears cached RPM.
On this machine both WMI selectors remained zero even while PM2 tachometers
measured fans spinning near 4000/4500 RPM. They are retained only as diagnostic
actions. Production `read_fan_rpm` explicitly selects the PM2 channel; both the
display and manual controller use its high/low/high coherent tachometer reads.

## Performance-mode acknowledgement regression

Treating all WMI responses as status-zero replies made successful mode writes
fail validation. The EC had already entered Office, but the command returned an
error before updating the application state and saved preference. When the UI's
five-second optimistic display expired, it showed the old Turbo state again.

The `set_power_mode` action now declares its channel and expected acknowledgement
separately for each mode. Read actions retain the channel's status-zero rule;
actual error statuses and another mode's echo remain rejected. Before committing
the cached mode, the driver confirms `get_power_mode` readback within 400 ms.
Readback failures cannot fall back to a cached Turbo value. The frontend clears
its optimistic display immediately when a command really fails.

Live regression checks passed Balance, Turbo, and Office switching and ten
consecutive one-second Office readbacks. The probe restores its original mode
on exit; the inspected machine started and ended in Office.

The inspected WMAA dispatcher does not expose charging limits or manual fans.
These controls instead use the declared `pmc_ec` channel, protocol
`xingyao_ptl_v1`, implemented in the shared HAL. Native access checks the actual
board, manufacturer and BIOS 1.00 independently of the JSON declaration.
No Tongfang or Baolongda EC register map is used.

## Integrated GPU telemetry (2026-11-03)

Contract 1.4.3 declares `gpu_telemetry_backend: intel_igcl_pmt` independently of
`discrete_gpu: false`. The dashboard previously zeroed every GPU reading on a
machine without a discrete GPU, bypassing the integrated graphics entirely.
The new read-only HAL loads the installed Intel runtime from its HKLM
`ControlApiPath`, verifies that the selected adapter is Intel integrated
graphics, and reads GPU clocks, available temperatures, and activity counters.
When IGCL has no temperature, it reads the GPU sensor through the existing
Intel PMT driver using Intel's published Panther Lake telemetry layout.
It does not enable discrete-GPU tuning or launch `nvidia-smi`.

Sampling follows the existing GPU telemetry demand flag. There is no periodic
hardware poll when no consumer requests GPU data, and the driver context is
released after ten seconds without requests. Samples are cached for one second;
driver failures clear readings and retry after thirty seconds. Activity uses
counter deltas, so the first sample has no utilization result. Unsupported
values remain unavailable rather than being replaced with CPU/package values.

Read-only validation on this machine, Intel Arc B390 with driver
`32.0.101.8357`, returned a live GPU frequency of 900 MHz. IGCL enumerated zero
temperature sensors and marked `gpuCurrentTemperature.bSupported` false.
Windows `D3DKMTQueryAdapterInfo` also returned zero temperature and frequency.
The PMT fallback successfully read a GPU temperature of 48 degrees Celsius.
This is the separate graphics `GT_MAX` sensor, not a CPU/package substitute.
The completed Rust probe then returned 56, 56 and 55 degrees Celsius in three
consecutive samples, all at 900 MHz, with `temperature_source: intel_pmt_gt_max`.

The installed PMT interface exposes two regions: GUID `0x03086100` (fixed,
186 DWORDs) and GUID `0x03086000` (normal, 838 DWORDs). Only the latter is
accepted. Intel's [PTL XML](https://github.com/intel/Intel-PMT/blob/main/xml/PTL/0/ptl_aggregator.xml)
defines `Container_15.GT_MAX` at bits 56..63, signed S8.7.0 Celsius. The
read-only Windows [PMT API](https://github.com/intel/compute-runtime/blob/master/level_zero/sysman/source/shared/windows/pmt/sysman_pmt_helper.h)
uses a QWORD index of 15 for this container. Discovery version, GUID, region
length and reply lengths are validated before accepting a temperature.
Unknown layouts and unavailable sensors retain `--` with an explanatory tooltip.
No new driver is installed and no physical-memory access is added.

The [IGCL programming guide](https://intel.github.io/drivers.gpu.control-library/prog.html)
documents that the runtime comes with the graphics driver and must not be bundled.

To inspect the same data independently, run from the project root:

```powershell
cargo run --quiet --manifest-path .\src-tauri\Cargo.toml -p openrevo-core --example intel_gpu -- --samples 3
```

This prints one JSON object per sample, including `temperature_source` to
distinguish `intel_igcl` from `intel_pmt_gt_max`. If neither source provides a
valid sensor value, `temperature_c` is null. The probe does not change clocks,
power limits, or fans.

### Integrated GPU power

Contract 1.4.3 also exposes `gpu_power_w` from IGCL's supported GPU energy
counter. The [IGCL specification](https://intel.github.io/drivers.gpu.control-library/Control/api.html)
defines average GPU power as the energy change in joules divided by the elapsed
driver timestamp in seconds. Both support flags, units and numeric types are
checked before using these counters. CPU package, system and total-card energy
counters are not substituted for the GPU domain.

The first sample establishes the baseline; the next telemetry update supplies
watts. Missing counters, resets, nonfinite numbers and sampling gaps over ten
seconds clear the reading until a new valid interval is available. A separate
`gpu_power_available` flag distinguishes valid zero watts from unavailable data.
The overview card and Custom telemetry row share this value. GPU readings below
1 W retain two decimal places; unsupported legacy APU telemetry retains its
existing shared-package label. The APU package total continues to show CPU
package power, which already includes the GPU, without adding GPU watts again.

A read-only four-sample probe returned null (baseline), 0.1224 W, 0.0984 W and
0.0230 W under the current desktop workload. The supplied historical AIDA64
screenshot separately reports `CPU GT Cores: 0.03 W`; it is not a simultaneous
comparison. These samples are not used to derive a power limit or calibration.

## Native CPU custom power (2026-11-02)

Contract 1.4.1 opts into the independent `cpu_power_backend: intel_rapl` and
exposes Custom alongside Office, Balance and Turbo. Custom is a software mode:
it selects the existing Turbo firmware preset once, then applies Intel package
PL1/PL2 through the installed, signed PawnIO IntelMSR module. The firmware still
reports mode 3; the HAL reports Custom only while the native CPU session owns it.
No fourth WMI performance command is invented.

The original CPU tuning controls probe the installed module for package power units, PL1/PL2,
enable flags, time windows and the firmware lock. The probe never writes to test
writability. An unlocked register is a candidate for writes; only successful
write/readback confirms a requested setting. Values retain fractional watts on
readback. PL2 occupies the upper 32-bit half, not bits 16–30. PL4, temperature
walls, voltage offsets and ratios remain unavailable.

The contract declares `cpu_power_max_watts: 50`, so both PL1 and PL2 have a
5–50 W adjustment range, with PL1 <= PL2. This ceiling was requested based on
the user's estimate of roughly 40 W sustained chassis cooling capacity; it is
a software limit, not a measured firmware or cooling limit. The current preset's
15/20 W settings do not reduce that adjustment range, and a Turbo preset above
50 W cannot raise it. Backend validation, the runtime probe, generated defaults
and the original UI use the same ceiling. Register locks and write/readback
validation still apply. Existing imported profiles above 50 W are rejected.
Other contracts without an explicit ceiling retain their firmware-budget policy.

To read current PL1/PL2 from an administrator PowerShell at the repository root:

```powershell
cargo run --quiet --manifest-path .\src-tauri\Cargo.toml -p openrevo-core --example cpu_power
```

After the first build, `src-tauri\target\debug\examples\cpu_power.exe` can be run
directly; add `--json` for machine-readable output. The probe uses the existing
PawnIO installation, only reads the current registers, and can run alongside
OpenRevo. It reports register values rather than another process's control state.

One mutex serializes CPU mode changes, profile edits and restoration. Writes
preserve time windows, clamp, lock and unrelated bits, verify readback, and
attempt rollback on failure. Selecting an ordinary mode restores only fields
still matching OpenRevo's last write, before selecting the firmware preset.
External changes are shown as an override and are not repeatedly overwritten.
Identical live edits do not rewrite registers. Updates to an active session use
the guarded update path, which cannot restart a canceled session. In the classic
UI, editing PL1/PL2 outside Custom applies the changed profile through the explicit
apply command; saving an inactive profile through the save API alone does not
enter Custom.

Profile changes persist the profile file and config as one serialized operation.
If replacing the config fails, the previous profile file is restored. A failed
save after a verified CPU write restores the prior owned limit fields and
controller session, and restores the previous firmware mode on initial entry.
The UI receives the error and does not claim a successful apply.

CPU custom power currently requires AC. Disconnecting it releases CPU control
and selects Balance. Normal exit and suspend release the saved settings; startup
and explicit OS resume attempt the saved Custom profile through the same guarded
path. Failed startup restoration falls back to Balance rather than claiming a
successful Custom mode. Forced process termination cannot run exit restoration.
The tuning page retains its original CPU/GPU/fan three-column layout. The fan
column selects the RPM editor for the declared PM2 backend and keeps the legacy
editor for other backends. Fixed RPM, RPM curves and software smoothing share
this column; there is no additional Overview fan entry. Unsupported controls
remain disabled. Legacy intensity curves are still migrated to RPM when saved;
CPU-only edits preserve the saved RPM and smoothing settings. Saving a fan policy
updates only the fan fields in the page, preserving pending CPU edits.

The host refuses to start `openrevo-plugin-power`'s companion agent on this native
backend. CPU writes also refuse to start while its default loopback port 18946
is listening. A separately launched agent must be closed before using the native
controller. This is not exclusive control over arbitrary external tuning tools;
readback/override detection remains necessary.

References: [plugin source, inspected commit 59cf948](https://github.com/xmsjx/openrevo-plugin-power/tree/59cf94881e597187adff022d10483b5027a5b39b),
[signed module ABI](https://github.com/namazso/PawnIO.Modules/blob/0.2.11/IntelMSR.p),
and [RAPL field definitions](https://github.com/torvalds/linux/blob/v6.12/drivers/powercap/intel_rapl_common.c).
The module binary is unchanged. This change has compile checks and simulated
register/state tests; no live CPU power writes or load tests were performed.

The existing `inpoutx64` port driver is required, with SHA-256
`f8965fdce668692c3785afa3559159f9a18287bc0d53abb21902895a8ecf221b`.
The backend opens its existing device; it does not install a driver or disable
Windows protections. The signed PawnIO CPU Package temperature reader is also
required before entering manual fan control.

## PM2 protocol and firmware evidence

The local DSDT maps PM2 data/status-command ports to `0x68`/`0x6c`.
Transactions serialize through the same named mutex as
[FanControl.MechrevoPtl](https://github.com/Kurisu934/FanControl.MechrevoPtl).
Every handshake has a one-second timeout, status `0xff` is rejected, and only
one persistent stale output byte may be consumed between transactions.
Partially sent commands are never automatically repeated.

Read-only extraction of the EC image from the host firmware window produced
SHA-256 `5989b63c442e909790e8da0314cc84564296a23f6c81bfc1eb92a4137ed91857`,
with header `ITE51300-SB-V.00120230800` and version
`ITE Tech. Inc. $INTEL PTL$MECHREVO$A0$VER:01.00.00$`.
This is the host-stored image, not a hash of independently read active EC flash.
The image and full machine dumps are kept out of distributed resources.

| Command | Parameter | Effect / local image evidence |
| --- | --- | --- |
| `D5` | `10` | Restore both fans to automatic control |
| `D6` | PWM byte | Shared PWM, `round(percent * 255 / 100)`; handler `C8CE` |
| `D7` / `D8` | RPM / 100 | Independent target RPM; handlers `C8FE` / `C922` |
| `D5` | `18/19`, `16/17` | Fan 0 / 1 low/high RPM bytes |
| `F1` | `10` / `11` | Disable / enable charging thresholds; dispatcher `C682` |
| `F1` | `12/13/14` | Read enable / charge boundary / discharge boundary |
| `F2` / `F3` | 0–100 | Set charge / discharge boundary, echo the written value; `C750` / `C790` |

The charging routine at `7A88` reads the two boundaries from EC-internal RAM
`0x800D08` and `0x800D09`. These are internal EC addresses, not host ACPI indices.
The second boundary may request **active discharge**. A charging cap sets it to
100%, sets the first boundary to 60% or 80%, then enables the policy. Selecting
100% disables the policy. Every update verifies both echoes and all three
readback values, and attempts to restore the previous state if an update fails.
These commands were verified from the PTL image and live behavior, independently
of superficially similar STX commands.

## Controls, validation and limits

### Configurable hardware fan ramp: interface audit (2026-11-01)

No usable interface for selecting a 100/300/500/1000-ms hardware fan ramp was
found in this machine's inspected BIOS 1.00 interfaces. This is distinct from
the EC's built-in incremental RPM feedback. `fan_ramp_rate` remains `false`:
the UI flag describes an adjustable feature, not the existence of feedback.

The audit used the local DSDT and the EC image hash documented above, not the
reference project's different BIOS 1.01 image. A read-only registry check
confirmed the current board is P916F-PTL with BIOS 1.00. No hardware settings,
unknown EC commands or memory writes were tried during this audit.

| Path | Evidence in the local firmware | Result |
| --- | --- | --- |
| OEM WMI / ACPI | DSDT `WMAA` routes version, fan readback, locks, touchpad, performance modes and keyboard timeout; its fan group only calls `GFNS` | No fan ramp setter |
| WMI extension | `OemWMIfun` / `OemWMIfunEx` have method IDs 1 / 2, but `WMAA` does not use the method-ID argument `Arg1`; it dispatches on the input buffer | The extension name does not establish an extra ramp command |
| PM2 target RPM | `D7` / `D8`, handlers `C8FE` / `C922`, set manual type and the RPM target byte; `8BF6` uses immediate error thresholds and PWM increments of 1, 2 or 4 | Built-in incremental control, without a ramp-period parameter in this path |
| PM2 direct PWM | `D6` and the manual type-2 branch at `8BF6` write the requested PWM output | No configurable time ramp in this path |
| Automatic thermal feedback | `8B18` reads record bytes 6 / 7 from the selected thermal table to delay PWM decrement / increment | Internal table counts; this routine is bypassed by the manual RPM branch and is not a discovered host ramp setter |
| Fan scheduling | `23D0`–`23EE` queues fan work using a literal modulo-100 condition; `4310`–`431A` dispatches through `B3E8` to `8D78` | No host-supplied period identified along this scheduling path; counts are not claimed to be milliseconds |

`ADDR_FAN_RAMP_RATE` in `build.rs` is a generated/fallback symbol from the
Tongfang register dictionary. The existing `set_fan_ramp_rate` writes that
Tongfang address; it is not a discovered PTL address or command. It must not be
enabled for PTL merely because the symbol exists.

This audit establishes that the reviewed WMI and PM2 fan-control paths do not
provide the requested setting. It does not prove that every undocumented
firmware path or future BIOS lacks one. The current PTL controller implements
the selectable transition in software, as described below. The declaration
uses `fan_software_smoothing=true` while retaining `fan_ramp_rate=false` for the
unavailable adjustable hardware ramp.

### Software smooth transition (2026-11-01)

The PTL panel provides a software smoothing switch, independent rise/fall rates
and a downward waiting time. It applies to both RPM curves and fixed manual
targets. Defaults are enabled, 1500 RPM/second rising, 300 RPM/second falling,
with a three-second wait before falling. Old profiles without smoothing fields
receive these defaults. Settings are stored in the active profile under
`ptl_rpm_fan_curves.smoothing` and restored at startup, including when the saved
curve is disabled. Editing these settings in automatic mode does not activate
manual control. The Tongfang hardware-ramp command is not used.

The existing 1-Hz control loop limits each channel's RPM command independently
using monotonic elapsed time, rounding movement to 100-RPM units while keeping
fractional movement for the next update. Settings describe RPM per second,
not a subsecond hardware command interval. Initial control and recovery from
firmware handover start from fresh PM2 tachometer readings. Live curve edits
with unchanged smoothing settings preserve the current transition and waiting
time. Idle time cannot accumulate credit for a later sudden speed jump.

At 85°C or higher, requested speed increases bypass the software ramp and
decreases are held until temperature falls. The full-speed boost command also
bypasses the ramp. The two-second start assistance and bounded stall recovery
remain downstream and can raise the command immediately; subsequent smoothing
continues from that raised command. The 90°C handover, missing-temperature
handover, communication failures, suspend and exit take precedence and clear
transition state. Curve recovery still requires the existing five-second
healthy window, then starts a fresh transition from firmware RPM.

The UI reports final requested RPM, the transition command and any start
assistance separately. Supported settings are 100–5000 RPM/second rising,
100–2000 RPM/second falling (in 100-RPM steps), and a 0–10-second downward
wait. Parameters are validated before activation or saving. Mocked tests cover
rate limits, timing, independent channels, live edits, start overrides, thermal
and sensor faults, firmware recovery and backward-compatible profile storage.
Real-machine smoothing verification is deferred at the user's request.

### RPM curve operation

The overview's “手动转速与风扇曲线” button opens a dedicated control panel without
requiring a fourth performance mode. Manual targets are independent, in
100-RPM steps: fan 0 accepts 1200–7500 RPM and fan 1 accepts 1000–7500 RPM.
Linked curves use the higher 1200-RPM floor. The manual sliders, curve editor
and backend validation use the same HAL limits; the declaration also records
both channel minima. Curves use CPU Package temperature
and issue independent RPM targets through `D7`/`D8`, with optional linking of
both curves. Values between temperature nodes are linearly interpolated and
rounded to 100 RPM. The 7500-RPM command ceiling is a target limit, not a
guarantee of the motors' attainable speed. Live tachometers remain separate
from requested targets.

Enabling control below 2000 RPM first requests 2000 RPM for two seconds, then
applies the requested low target. Every update checks both PM2 tachometers.
Unexpected zero RPM triggers the same assistance on that fan only. A failed
two-second assisted start, three seconds of persistent zero RPM at higher
targets, or a third restart within 30 seconds hands control back to firmware.
These timers do not block the temperature watchdog. The panel distinguishes
the requested target from the RPM command temporarily used for starting.
Curve protection retains its existing five-second recovery window; fixed
manual control returns to automatic after a fault.

The panel edits RPM directly: a 40–50°C plateau at 2200 RPM requests 2200 RPM
from each selected fan. The previous PWM curve used byte 64/255 at 25%, which
measured around 3000 RPM and did not request 2200 RPM. Existing percentage
profiles retain their temperature knots and relative intensity when migrated:
25% or lower becomes 2200 RPM, 100% becomes 7500 RPM, with linear scaling
between them. This is a migration policy, not a physical PWM-to-RPM calibration.
The original percentage arrays remain intact for other controllers. New curves
are stored in `ptl_rpm_fan_curves` in the active profile file used at startup.

The software curve runs at 1 Hz even while the main window is hidden. A missing
temperature, temperature at or above 90°C, a communication error, a scheduling
gap longer than five seconds, or suspend temporarily hands the fans to firmware.
The requested curve is retained and the status reports `resume_pending=true`.
After temperature falls to 85°C or lower and valid temperature samples and PM2
compatibility queries remain healthy for five continuous seconds, the controller
automatically resumes that curve using the current temperature. Any new fault,
temperature above 85°C, or edit during protection resets this recovery window.
Sleep pauses recovery until wake. Failed firmware restoration is reported as
`recovering` and retried before starting the recovery window.

Explicitly selecting factory automatic control cancels the requested curve
before any I/O, even if that restoration attempt fails; it cannot automatically
resume later. Shutdown and normal exit also cancel resumption. Fixed manual
RPM targets return to automatic on faults or suspend and do not resume themselves.
The overview badge and control panel show actual control mode, recovery state,
CPU Package temperature and requested RPM rather than inferring mode from boost.
Curves therefore require OpenRevo to remain running. There is no independent
watchdog for a process crash or forced termination; restoration cannot be
guaranteed in that case. Actual sleep/wake cycling and sustained thermal loads
have not been tested in this session.

Live checks on BIOS 1.00:

- Requested 4000/4500 RPM, shared-core backend measured 3913/4445 RPM.
- After switching production tachometer reads to PM2, the HAL display returned
  3963/4436 RPM, matching the controller's own measurement.
- The previous constant 50% PWM curve measured 5337/4922 RPM; encoding was 128/255.
- Low-speed 2000/2000 targets measured 1962/1906 RPM; the minimum 25% PWM
  curve previously kept both motors running at 2953/2594 RPM.
- 60% / 80% / 100% charging policies all passed hardware readback.
- With battery charge above the 80% boundary, charging changed from 7730 mW to
  0 mW; `Charging=false`, `Discharging=false`, external power remained online.
- Test cleanup restored automatic fans and the original disabled charging cap.

### Minimum-speed measurements (2026-10-31)

The installed BIOS 1.00 machine was tested in Office mode, with OpenRevo exited
and the PM2 mutex held for each entire experiment. RPM targets were tested first,
then direct PWM was reduced to distinguish motor sustaining speed from RPM-loop
hunting. The companion fan remained in RPM control: 4500 RPM for fan 0's test,
6500 RPM for fan 1's final confirmation. These are host-visible PWM bytes, not
an electrical duty-cycle measurement. No firmware or production RPM limit was
changed.

| Fan | Lowest sustained PWM byte | Mean RPM | Stable RPM range | Next lower byte |
| --- | --- | --- | --- | --- |
| Fan 0 (main) | 19 / 255 | 674 | 668–680 | 18: stopped |
| Fan 1 (secondary) | 18 / 255 | 391 | 379–397 | 17: stopped |

Each minimum duty was held for 60 samples, approximately 61 seconds. Statistics
above use the final 52 samples after eight seconds of settling; CPU Package
readings during those stable segments were 42–66°C / 40–58°C respectively.
Both fans failed to restart within five seconds at their minimum sustaining
duty after three consecutive zero-RPM samples. A sustaining minimum therefore
does not establish a reliable starting command.

The RPM-loop experiment separately confirmed fan 0's 1100-RPM target at about
1080 RPM, including three stop/restart trials; its 1000-RPM target hunted.
Fan 1 initially held about 729 RPM at an 800-RPM target, but failed the longer
confirmation after visiting a lower unstable target (583–1080 RPM in the final
eight samples). The physical sustaining values above are therefore not validated
minimum targets for RPM curves. A later test of the two-second 2000-RPM start
assistance followed by an 1100-RPM main-fan target produced 766–1237 RPM
([startup trace](measurements/ptl-rpm-startup-1100-20261031.csv)),
so the configurable main-fan floor was set to 1200 RPM. The secondary floor is
1000 RPM, above the unstable 800-RPM operating point. The original 2200-RPM
default curve and legacy percentage migration are retained.

Unit checks cover the new channel limits, linked curves, timed start assistance,
independent restart attempts, failed starts and persistent stalls. The follow-up
three-start-per-fan verification at the configured 1200/1000-RPM floors was
interrupted during thermal preflight by an 85°C Package reading; this run is
not recorded as a successful hardware validation. Automatic restoration
succeeded on both aborted follow-up runs.

CPU Package temperature and both tachometers were sampled every second. Earlier
runs stopped at conservative temperature limits and successfully restored
automatic control. The final secondary-fan run stopped on a single reading at
75°C or three consecutive readings at 70°C, with the companion fan increased to
6500 RPM. The final experiment completed, restored automatic fans, and the
read-only post-check confirmed Office mode and the original 80% charging cap.

The [measurement samples](measurements/ptl-minimum-fan-speed-20261031.csv) retain
descending duties, stopped-fan boundaries, confirmation and restart observations.
`steady_confirmation=true` identifies the samples used for the table. Times
are relative to the separate fan 0 / fan 1 runs. These results describe this
machine under the tested conditions; they do not establish a universal motor
specification for every PTL unit or temperature.

Protocol tests cover partial-write non-retry, charge-update rollback,
discharge-boundary ordering and torn tachometer samples. Mocked controller tests
cover independent RPM interpolation, legacy-profile conversion, overheat
hysteresis, sensor failure, failed communications and restoration, scheduling
gaps, user cancellation and sleep/wake resumption. Profile tests verify RPM
serialization without changing legacy units. Valid curves selected during a
sensor or communication fault are saved as pending intent; the UI reports the
protection reason instead of claiming the curve is running. Invalid input is
rejected without replacing the active policy. A WMI mailbox alone never
selects the Baolongda controller.

Intel CPU Package temperature uses the separately documented PawnIO sensor.
Read-only checks can be repeated with:

```powershell
cargo run --offline -p openrevo-core --example ptl_contract_probe
```

The separate `ptl_control_probe -- --verify-controls` example performs bounded
reversible real-hardware tests; its default invocation only reads state.

Power-mode regression can be checked with
`cargo run --offline -p openrevo-core --example ptl_power_mode_probe -- --verify-switches`.
Without `--verify-switches`, that probe only reads the active hardware mode.

`cargo run --offline -p openrevo-core --example ptl_control_probe -- --sample-fans`
samples CPU Package temperature and both tachometers eight times without
changing fan policy or charging settings. This can run alongside an active
software curve. A historical sample while the panel reported PWM 25% measured
3071–3084 / 2746–2753 RPM on this machine.
