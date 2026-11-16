export interface NavigationLocation {
  readUrl: () => string;
  replaceUrl: (url: string) => void;
}

export interface Navigation {
  route: () => string;
  change: (url: string) => string | null;
  protect: (allow: () => boolean) => () => void;
  replaceHash: (expected: string, replacement: string) => boolean;
}

function hash(url: string): string {
  return new URL(url).hash || "#/";
}

export function createNavigation(location: NavigationLocation): Navigation {
  let acceptedUrl = location.readUrl();
  const guards = new Set<() => boolean>();

  function change(url: string): string | null {
    if (url !== location.readUrl() || url === acceptedUrl) return null;
    for (const allow of guards) {
      if (!allow()) {
        location.replaceUrl(acceptedUrl);
        return null;
      }
    }
    acceptedUrl = url;
    return hash(acceptedUrl);
  }
  function protect(allow: () => boolean): () => void {
    guards.add(allow);
    return () => {
      guards.delete(allow);
    };
  }
  function replaceHash(expected: string, replacement: string): boolean {
    if (location.readUrl() !== acceptedUrl) return false;
    if (hash(acceptedUrl) !== expected && hash(acceptedUrl) !== replacement) return false;
    const url = new URL(acceptedUrl);
    url.hash = replacement;
    location.replaceUrl(url.href);
    acceptedUrl = url.href;
    return true;
  }
  return { route: () => hash(acceptedUrl), change, protect, replaceHash };
}
