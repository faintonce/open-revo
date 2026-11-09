import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

/**
 * Structural guard for the commit point in `ec_mode`.
 *
 * `hardware_confirmed` is only meaningful if every write that decides the mode lands *before*
 * `committed.set(true)` and the trigger latch is the last of them. When Custom and Turbo share
 * `ADDR_MODE_INDEX`/`ADDR_TRIGGER_BYTE`, the only distinguishing write is `ADDR_CUSTOM_MODE`
 * (`set_custom_mode_ec`), so a reorder there silently turns "still Custom" into "confirmed
 * Turbo". The register writes have no injectable seam, so this asserts the source order.
 */
const files = [
  '../src-tauri/src/system/power/ec_mode.rs',
  '../src-tauri/crates/openrevo-core/src/system/power/ec_mode.rs',
];

for (const file of files) {
  test(`mode identity is written before the latch and the commit point (${file})`, () => {
    const text = readFileSync(new URL(file, import.meta.url), 'utf8');
    const positions = (pattern) => [...text.matchAll(pattern)].map((match) => match.index);

    const commits = positions(/committed\.set\(true\);/g);
    const identities = positions(/acknowledged\(\s*wmi_ec::set_custom_mode_ec\((?:true|false)\),/g);
    const latches = positions(/write_trigger\(wmi_ec::ADDR_TRIGGER_BYTE/g);

    // One HAL branch plus the four EC modes.
    assert.equal(commits.length, 5, 'unexpected number of commit points');
    assert.equal(identities.length, 4, 'every EC mode must set the custom-mode identity');
    assert.equal(latches.length, 4, 'every EC mode must latch the trigger');

    for (let mode = 0; mode < latches.length; mode += 1) {
      assert.ok(
        identities[mode] < latches[mode],
        `mode ${mode + 1}: ADDR_CUSTOM_MODE must be written before the trigger latch`,
      );
      assert.ok(
        latches[mode] < commits[mode + 1],
        `mode ${mode + 1}: the commit point must follow the trigger latch`,
      );
    }
  });
}
