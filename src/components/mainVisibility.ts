import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { subscribeWidgetTelemetry } from './widgetTelemetry';

export const mainVisibility = {
  listenVisibility: (handler: (visible: boolean) => void) => listen<boolean>('main-telemetry-visibility', (event) => handler(event.payload)),
  isVisible: async () => (await invoke<{ main_polling: boolean }>('get_telemetry_heartbeat_status')).main_polling,
};

export function subscribeVisiblePolling<T>(read: () => Promise<T>, receive: (state: T) => void, intervalMs: number) {
  return subscribeWidgetTelemetry({ ...mainVisibility, readSnapshot: read, listenTelemetry: async () => () => {} }, receive, intervalMs);
}
