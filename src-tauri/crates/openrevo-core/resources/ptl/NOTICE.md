The PTL PM2 port protocol, bounded stale-output synchronization and coherent fan
tachometer sampling in `src/hal/ptl_pmc.rs` are adapted from
[FanControl.MechrevoPtl](https://github.com/Kurisu934/FanControl.MechrevoPtl),
copyright (c) 2026 Kurisu934, under the MIT license in `LICENSE` beside this file.

No inpout kernel driver binary or vendor firmware image is redistributed here.
The backend requires the existing driver with the explicitly verified SHA-256.
Battery commands were independently checked against this machine's EC 01.00.00
image and live charge-limit readback; see `docs/xingyao-p916f-ptl.md` in the repository.
