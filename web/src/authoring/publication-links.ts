export function publicStudyHash(id: string, revision: number | null = null): string {
  let path = `#/studies/${encodeURIComponent(id)}`;
  if (revision !== null) path += `/revisions/${revision}`;
  return path;
}

export function publicMediaPath(id: string, revision: number): string {
  return `/api/studies/${encodeURIComponent(id)}/revisions/${revision}/media`;
}
