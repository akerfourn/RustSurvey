import { useLayoutEffect, useRef, useState } from "preact/hooks";
import { createAutosave, type AutosaveState } from "./autosave.ts";

export function useAutosave(state: AutosaveState, save: () => void): void {
  const latestSave = useRef(save);
  latestSave.current = save;
  const [autosave] = useState(() =>
    createAutosave(
      {
        now: () => performance.now(),
        schedule: (action, delay) => {
          const timer = window.setTimeout(action, delay);
          return () => window.clearTimeout(timer);
        },
      },
      () => latestSave.current(),
    ),
  );
  useLayoutEffect(() => autosave.update(state));
  useLayoutEffect(() => () => autosave.dispose(), [autosave]);
}
