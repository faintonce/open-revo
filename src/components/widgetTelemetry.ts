type Unlisten = () => void;
interface TelemetrySource<T> {
  readSnapshot(): Promise<T>;
  listenTelemetry(handler: (state: T) => void): Promise<Unlisten>;
  listenVisibility(handler: (visible: boolean) => void): Promise<Unlisten>;
  isVisible(): Promise<boolean>;
}

export function subscribeWidgetTelemetry<T>(source: TelemetrySource<T>, receive: (state: T) => void, intervalMs = 5000): Unlisten {
  let disposed = false;
  let visible = false;
  let generation = 0;
  let inFlight = false;
  let lastTelemetry: number | null = null;
  let timer: ReturnType<typeof setInterval> | undefined;
  const unlisteners: Unlisten[] = [];

  const read = async (force = false) => {
    if (disposed || !visible || inFlight || (!force && lastTelemetry !== null && Date.now() - lastTelemetry < intervalMs)) return;
    const requestedGeneration = generation;
    inFlight = true;
    try {
      const state = await source.readSnapshot();
      if (!disposed && visible && generation === requestedGeneration) receive(state);
    } catch (error) {
      console.warn('Widget snapshot failed:', error);
    } finally {
      inFlight = false;
    }
  };

  const setVisible = (next: boolean) => {
    if (disposed) return;
    generation += 1;
    if (visible === next) return;
    visible = next;
    clearInterval(timer);
    if (visible) {
      lastTelemetry = null;
      void read(true);
      timer = setInterval(() => { void read(); }, intervalMs);
    }
  };

  const attach = async (subscription: Promise<Unlisten>) => {
    const unlisten = await subscription;
    if (disposed) unlisten();
    else unlisteners.push(unlisten);
  };

  void (async () => {
    await attach(source.listenVisibility(setVisible));
    if (disposed) return;
    void attach(source.listenTelemetry((state) => {
      lastTelemetry = Date.now();
      if (!disposed && visible) receive(state);
    })).catch(console.warn);
    const requestedGeneration = generation;
    const initiallyVisible = await source.isVisible();
    if (!disposed && generation === requestedGeneration) setVisible(initiallyVisible);
  })().catch(console.warn);

  return () => {
    disposed = true;
    clearInterval(timer);
    for (const unlisten of unlisteners) unlisten();
  };
}
