/** Debounce previews, flush on release/blur/close, and serialize writes per character. */
export function createScaleSaver(save: (id: string, scale: number) => Promise<void>, onError: (error: unknown) => void) {
  const pending = new Map<string, { value: number; timer: ReturnType<typeof setTimeout> }>();
  const chains = new Map<string, Promise<void>>();
  const requested = new Map<string, number>();
  function flush(id?: string): void {
    for (const key of id ? [id] : [...pending.keys()]) {
      const item = pending.get(key);
      if (!item) continue;
      clearTimeout(item.timer);
      pending.delete(key);
      if (requested.get(key) === item.value) continue;
      requested.set(key, item.value);
      const task = (chains.get(key) ?? Promise.resolve()).then(() => save(key, item.value)).catch(error => {
        if (requested.get(key) === item.value) requested.delete(key);
        onError(error);
      });
      chains.set(key, task);
      void task.finally(() => { if (chains.get(key) === task) chains.delete(key); });
    }
  }
  return {
    schedule(id: string, value: number) {
      const previous = pending.get(id);
      if (previous) clearTimeout(previous.timer);
      pending.set(id, { value, timer: setTimeout(() => flush(id), 260) });
    },
    flush,
  };
}
