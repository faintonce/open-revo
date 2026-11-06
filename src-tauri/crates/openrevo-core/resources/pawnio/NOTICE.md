# PawnIO IntelMSR module

`IntelMSR.bin` is an unmodified, signed module from PawnIO.Modules 0.2.11,
copyright (C) 2025 namazso. License: LGPL-2.1-or-later (see `COPYING`).

- Release: https://github.com/namazso/PawnIO.Modules/releases/tag/0.2.11
- Complete corresponding source: https://github.com/namazso/PawnIO.Modules/archive/refs/tags/0.2.11.tar.gz
- Module source: https://github.com/namazso/PawnIO.Modules/blob/0.2.11/IntelMSR.p
- Module SHA-256: `d6ed85d65ab17a22f813ef98207d6d537155ee2ded5976a21cb48413c9b92e5f`
- Release archive SHA-256: `43608cb89bc84247fef1368a139013f7d043e17db6d6c8dfc9b46bf0905a81f4`

OpenRevo embeds this module for Intel package temperature and, on model contracts
opting into `intel_rapl`, CPU package PL1/PL2 control. Reads use `ioctl_read_msr`;
the power backend uses `ioctl_write_msr` only for the package power-limit register.
It preserves the existing time windows, clamp bits and unrelated fields, checks
readback and restores settings when relinquishing control. OC mailbox, voltage,
ratio and PL4 writes are not exposed by this backend. The separately installed
signed PawnIO driver verifies the module signature. OpenRevo does not install the
driver or change its access permissions.

Install the official signed driver from https://pawnio.eu and run OpenRevo as
administrator to enable Intel CPU Package temperatures. No AIDA64 process is
required. If no genuine CPU sensor is available, temperature is reported as
unavailable instead of substituting an ACPI thermal zone or a fixed value.
