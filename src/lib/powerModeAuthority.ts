/**
 * Boot-time authority rule for `get_app_config` relative to live hardware telemetry.
 *
 * `bootSync` fires once on mount, but the answer can arrive after a telemetry snapshot or after
 * the user already changed something. Both are newer than the config file, so the config must not
 * overwrite them. The decision is a pure function so it can be unit tested without React: the
 * regression it guards against (a stale config resetting a confirmed choice) only shows up when
 * the effect re-runs, which the component tests cannot provoke.
 */

/** A value the user just changed, protected until `expiry` (epoch ms). */
export interface SelectionLock {
  expiry: number;
}

/**
 * Returns true when the persisted config may set a value that telemetry/UI also own.
 *
 * @param capturedRevision snapshot counter captured before the config request was sent
 * @param currentRevision snapshot counter read after the answer arrived
 * @param lock last user-changed value for this field, if any
 * @param now current wall clock, injected for tests
 */
export function configIsAuthoritative(
  capturedRevision: number,
  currentRevision: number,
  lock: SelectionLock | undefined,
  now: number,
): boolean {
  const telemetryArrivedDuringRequest = capturedRevision !== currentRevision;
  const selectionStillPending = lock !== undefined && now < lock.expiry;
  return !telemetryArrivedDuringRequest && !selectionStillPending;
}
