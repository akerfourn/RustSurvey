import { useLayoutEffect, useState } from "preact/hooks";
import { createNavigation, type Navigation } from "./controller.ts";

export function useHashRoute(): { route: string; navigation: Navigation } {
  const [navigation] = useState(() =>
    createNavigation({
      readUrl: () => window.location.href,
      replaceUrl: (url) => window.history.replaceState(window.history.state, "", url),
    }),
  );
  const [route, setRoute] = useState(navigation.route);
  useLayoutEffect(() => {
    function handleRoute(event: HashChangeEvent): void {
      const next = navigation.change(event.newURL);
      if (next !== null) setRoute(next);
    }
    window.addEventListener("hashchange", handleRoute);
    return () => window.removeEventListener("hashchange", handleRoute);
  }, [navigation]);
  return { route, navigation };
}
