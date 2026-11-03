# Xingyao 14 P916F-PTL

## Supported hardware

This contract covers MECHREVO `XINGYAO Series-P916F-PTL`, Intel Core Ultra
X7 358H, BIOS 1.00. STX, LNL and other BIOS versions are outside the validated
scope. The exact PTL contract takes priority over the older P916F wildcard.
Manufacturer and BIOS checks apply during matching; an exact PTL identity with
an unvalidated BIOS receives a monitoring-only contract without write actions.
Native PM2 access independently verifies the machine identity and BIOS.

| Function | Backend / behavior |
| --- | --- |
| Office / Balance / Turbo | OEM WMI with acknowledgement and mode readback |
| Charging limits | PM2 thresholds: 60%, 80%, or policy disabled at 100% |
| Fan feedback and targets | Independent PM2 tachometers and target RPM |
| Fan curves | CPU Package temperature to RPM, with protection and recovery |
| Fan smoothing | Software backend behind the original UI controls |
| CPU temperature | Genuine Intel package DTS through installed PawnIO |
| CPU custom power | Opt-in Intel RAPL, 5 <= PL1 <= PL2 <= 50 W, AC required |
| Integrated GPU telemetry | Installed Intel IGCL runtime and validated PMT layout |

The tuning page keeps its original layout, percentage curve editor and
100/300/500/1000ms smoothing selector. Unsupported CPU controls remain disabled
in their existing positions. Ratios, undervolting, PL4, temperature-wall tuning
and discrete-GPU tuning are not implemented for this contract.

## Runtime dependencies and attribution

PM2 access requires an existing `inpoutx64` driver with SHA-256
`f8965fdce668692c3785afa3559159f9a18287bc0d53abb21902895a8ecf221b`.
CPU sensor/control access requires the installed signed PawnIO driver and
administrator access. OpenRevo does not install these drivers or change their
permissions. Manual fan control requires a genuine package-temperature sensor.
Missing readings remain unavailable instead of using ACPI zones or fixed values.

The unmodified signed IntelMSR module is embedded for PawnIO; its origin and
license are recorded in `src-tauri/crates/openrevo-core/resources/pawnio/`.
PM2 protocol attribution and the MIT license are retained in the neighboring
`resources/ptl/` directory. Vendor firmware images and the inpout driver binary
are not bundled. IGCL is loaded from the installed Intel graphics driver.

## Protocol and controller behavior

The OEM interface is `root\wmi:OemWMIMethod.OemWMIfun`, with a 64-byte input
and 256-byte output. Read replies require status zero. Performance-mode writes
use `03 0A mode` and acknowledge Office / Balance / Turbo with byte zero equal
to `0x94` / `0x95` / `0x96`. Mode readback must agree within 400ms before the
application commits the new state. A successful WMI transport alone is
insufficient. WMI fan selectors require separate calls; on the inspected machine
they returned zero while the motors were spinning, so production feedback uses
coherent high/low/high PM2 tachometer reads.

PM2 transactions use the shared named mutex, bounded handshakes and validated
responses. Partially sent commands are not retried. Charging updates keep the
active-discharge boundary at 100%, verify echoes and all threshold readbacks,
and attempt to restore the prior state on failure. These are PTL-specific
commands, independently checked against the local EC image and live behavior.
The implementation does not reuse Tongfang or Baolongda register maps.

CPU Custom mode uses the firmware Turbo mode plus software ownership of the
RAPL limit fields. Ordinary presets select firmware modes; they do not promise
fixed PL1/PL2 values. CPU writes preserve time windows, clamp/lock and unrelated
bits, verify readback, and restore only fields still owned by OpenRevo. Failed
profile persistence restores the previous profile and CPU ownership/settings.
Live updates cannot restart a session canceled by a mode switch. AC loss,
suspend and normal exit release CPU control; failed startup restoration falls
back to an acknowledged Balance mode. The native backend blocks the legacy
power-plugin companion agent to avoid competing writers.

## Fan policy

The native target ranges are 1200-7500 RPM for fan 0 and 1000-7500 RPM for fan 1,
in 100-RPM steps; linked curves use the higher minimum. These are configured
target limits, not guaranteed motor speeds. The original percentage editor maps
25% or lower to 2200 RPM and 100% to 7500 RPM, with interpolation between them.
This preserves legacy curve intensity rather than claiming a physical PWM
calibration. CPU-only profile edits preserve saved RPM data.

Hardware ramp configuration remains unsupported (`fan_ramp_rate=false`). The
separate `fan_software_smoothing=true` capability routes the existing ramp
command to software: each channel moves by 100 RPM per selected interval,
using actual monotonic elapsed time, with a three-second wait before lowering
speed. Active checks are bounded to 100-1000ms. Legacy UI fields and the RPM
policy are saved together and restored at startup. Configuring smoothing in
automatic mode does not start manual control; failed saves restore prior settings.

At 85 C, increases bypass smoothing and decreases are held. At 90 C, missing
temperature, communication failure, a scheduling gap over five seconds or suspend,
control passes to firmware. A requested curve resumes only after temperature is
at most 85 C and sensors/communication stay healthy for five continuous seconds.
Explicitly disabling the curve cancels recovery. Fixed manual targets do not
resume after faults. Targets below 2000 RPM receive two seconds of start
assistance; persistent or repeated stalls return control to firmware.
Curves require the process to remain running; forced termination cannot execute
restoration.

## Integrated GPU telemetry

`gpu_telemetry_backend: intel_igcl_pmt` is independent of discrete-GPU tuning.
IGCL supplies supported clocks, activity and energy counters; GPU watts use
energy/time deltas, with no value until a valid second sample. Package power is
not double-counted with integrated-GPU power.

When IGCL has no temperature sensor, the read-only PMT fallback accepts GUID
`0x03086000` and the validated region layout. `Container_15.GT_MAX`, bits 56-63,
is the separate graphics temperature sensor. Unknown layouts and invalid
readings remain unavailable. See Intel's
[PTL telemetry definition](https://github.com/intel/Intel-PMT/blob/main/xml/PTL/0/ptl_aggregator.xml)
and [IGCL guide](https://intel.github.io/drivers.gpu.control-library/prog.html).

## Validation and remaining coverage

Previous checks on this machine verified mode switching/readback, charging
limits, PM2 fan control/feedback, and integrated GPU telemetry. For example,
4000/4500 RPM targets measured 3913/4445 RPM; the HAL display agreed with PM2.
At charge above the 80% boundary, the charging policy stopped charging while
external power stayed online, without requesting active discharge.

Automated regression coverage uses simulated registers/transports, controller
state tests, profile/storage tests and mocked UI commands. It covers partial
writes, rollback, mode acknowledgements, RPM conversion, all four UI smoothing
intervals, thermal/sensor faults, recovery and cancellation. Protocol constants
and native access remain confined to the HAL and hardware vault.

Native CPU power writes/load testing, complete cold-start verification of the
1200/1000-RPM floors, software smoothing on hardware, and sustained thermal and
sleep/wake tests remain pending. Low-speed startup verification was interrupted
by temperature protection; it is not counted as a successful validation. The
legacy 2200-RPM default curve remains unchanged. No hardware writes were
performed during PR preparation. Raw experiment traces and standalone diagnostic
probes are excluded from the integration change; the validation summary above
retains the evidence and limits relevant to maintenance.
