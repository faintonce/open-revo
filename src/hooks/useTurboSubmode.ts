import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface TurboSubmodeState {
  supported: boolean;
  silent: boolean;
}

const INITIAL_STATE: TurboSubmodeState = { supported: false, silent: false };

function normalize(payload: unknown): Partial<TurboSubmodeState> | null {
  if (!payload || typeof payload !== 'object') return null;
  const value = payload as Record<string, unknown>;
  const next: Partial<TurboSubmodeState> = {};
  if (typeof value.supported === 'boolean') next.supported = value.supported;
  if (typeof value.silent === 'boolean') next.silent = value.silent;
  return Object.keys(next).length > 0 ? next : null;
}

function describeError(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  return String(err);
}

/**
 * Shared 狂暴子模式 state for every WebView surface.
 *
 * One initial query, then event-driven updates. Applying always goes through
 * the single `apply_turbo_profile` command (which also ensures mode 3), so no
 * caller needs to send `set_power_mode` plus a fixed sleep.
 */
export function useTurboSubmode() {
  const [state, setState] = useState<TurboSubmodeState>(INITIAL_STATE);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busyRef = useRef(false);
  // Bumped whenever newer state is adopted; lets us drop stale query responses
  // that were already in flight when an event arrived.
  const revisionRef = useRef(0);

  const merge = useCallback((payload: unknown) => {
    const next = normalize(payload);
    if (!next) return;
    revisionRef.current += 1;
    setState((prev) => ({ ...prev, ...next }));
  }, []);

  useEffect(() => {
    let alive = true;
    const unlisteners: Array<() => void> = [];
    const track = (pending: Promise<() => void>) => {
      pending
        .then((fn) => {
          if (alive) unlisteners.push(fn);
          else {
            try { fn(); } catch { /* ignore */ }
          }
        })
        .catch(() => { });
    };
    const refresh = () => {
      const revision = revisionRef.current;
      invoke<TurboSubmodeState>('get_turbo_submode_state')
        .then((value) => {
          if (!alive || revision !== revisionRef.current) return;
          merge(value);
        })
        .catch(() => { });
    };

    refresh();
    track(listen<TurboSubmodeState>('turbo-submode-changed', (event) => merge(event.payload)));
    // Physical Q-key / tray switches only report the power mode. Query the
    // submode once immediately (no fixed compensation timer).
    track(listen<number>('physical-mode-switched', (event) => {
      if (Number(event.payload) !== 3) return;
      refresh();
    }));

    return () => {
      alive = false;
      for (const fn of unlisteners) {
        try { fn(); } catch { /* ignore */ }
      }
    };
  }, [merge]);

  const apply = useCallback(async (silent: boolean) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError(null);
    try {
      const value = await invoke<TurboSubmodeState>('apply_turbo_profile', { silent });
      merge(value);
    } catch (err) {
      setError(describeError(err));
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }, [merge]);

  return { ...state, busy, error, apply };
}
