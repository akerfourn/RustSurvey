export const AUTOSAVE_IDLE_MS = 2000;
export const AUTOSAVE_MAX_WAIT_MS = 10000;

export interface AutosaveClock {
  now: () => number;
  schedule: (action: () => void, delay: number) => () => void;
}

export interface AutosaveState {
  editVersion: number;
  attemptedVersion: number;
  dirty: boolean;
  enabled: boolean;
}

export interface Autosave {
  update: (state: AutosaveState) => void;
  dispose: () => void;
}

export function createAutosave(clock: AutosaveClock, save: () => void): Autosave {
  let observedVersion = 0;
  let attemptedVersion = 0;
  let firedVersion = 0;
  let firstEditAt: number | null = null;
  let lastEditAt: number | null = null;
  let cancel: (() => void) | null = null;
  let disposed = false;

  function cancelTimer(): void {
    cancel?.();
    cancel = null;
  }
  function update(state: AutosaveState): void {
    if (disposed) return;
    const now = clock.now();
    if (observedVersion !== state.editVersion) {
      observedVersion = state.editVersion;
      lastEditAt = now;
      firstEditAt ??= now;
    }
    if (attemptedVersion !== state.attemptedVersion) {
      attemptedVersion = state.attemptedVersion;
      firstEditAt = lastEditAt;
    }
    cancelTimer();
    if (!state.dirty || state.editVersion <= Math.max(attemptedVersion, firedVersion)) {
      firstEditAt = null;
      lastEditAt = null;
      return;
    }
    firstEditAt ??= now;
    lastEditAt ??= now;
    if (!state.enabled) return;
    const deadline = Math.min(firstEditAt + AUTOSAVE_MAX_WAIT_MS, lastEditAt + AUTOSAVE_IDLE_MS);
    cancel = clock.schedule(
      () => {
        cancel = null;
        firedVersion = state.editVersion;
        firstEditAt = null;
        save();
      },
      Math.max(0, deadline - now),
    );
  }
  function dispose(): void {
    disposed = true;
    cancelTimer();
  }
  return { update, dispose };
}
