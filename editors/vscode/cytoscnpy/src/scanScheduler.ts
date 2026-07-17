export class DebouncedTaskMap {
  private readonly timers = new Map<string, NodeJS.Timeout>();

  schedule(key: string, delayMs: number, task: () => void): void {
    const existing = this.timers.get(key);
    if (existing) {
      clearTimeout(existing);
    }
    const timer = setTimeout(() => {
      this.timers.delete(key);
      task();
    }, delayMs);
    this.timers.set(key, timer);
  }

  dispose(): void {
    for (const timer of this.timers.values()) {
      clearTimeout(timer);
    }
    this.timers.clear();
  }
}

export class CoalescingTaskQueue {
  private pending: (() => Promise<void>) | undefined;
  private running: Promise<void> | undefined;

  run(task: () => Promise<void>): Promise<void> {
    this.pending = task;
    if (!this.running) {
      this.running = this.drain().finally(() => {
        this.running = undefined;
      });
    }
    return this.running;
  }

  private async drain(): Promise<void> {
    while (this.pending) {
      const task = this.pending;
      this.pending = undefined;
      await task();
    }
  }
}
