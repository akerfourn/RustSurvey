import { useContext, useLayoutEffect, useRef } from "preact/hooks";
import { NavigationContext } from "./context.ts";

export function useUnsavedChanges(active: boolean, hasUnsavedChanges: () => boolean): void {
  const navigation = useContext(NavigationContext);
  const latest = useRef(hasUnsavedChanges);
  latest.current = hasUnsavedChanges;
  useLayoutEffect(() => {
    if (!active) return;
    function allowNavigation(): boolean {
      if (!latest.current()) return true;
      return window.confirm(
        "Des modifications ne sont pas encore confirmées par le serveur. Quitter cette saisie quand même ?",
      );
    }
    function preventUnload(event: BeforeUnloadEvent): void {
      if (latest.current()) event.preventDefault();
    }
    const release = navigation?.protect(allowNavigation);
    window.addEventListener("beforeunload", preventUnload);
    return () => {
      release?.();
      window.removeEventListener("beforeunload", preventUnload);
    };
  }, [active, navigation]);
}
