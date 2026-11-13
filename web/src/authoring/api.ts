import type { Definition, Session, StoredStudy, StudySummary, ValidationIssue } from "./model.ts";
import { decodeDefinition, decodeSourceInfo } from "./definition-codec.ts";
export { decodeDefinition } from "./definition-codec.ts";

export function object(value: unknown): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value))
    throw new Error("Réponse du serveur invalide.");
  return value as Record<string, unknown>;
}
function text(value: unknown): string {
  if (typeof value !== "string") throw new Error("Texte du serveur invalide.");
  return value;
}
function number(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value))
    throw new Error("Nombre du serveur invalide.");
  return value;
}
function array<T>(value: unknown, decode: (item: unknown) => T): T[] {
  if (!Array.isArray(value)) throw new Error("Liste du serveur invalide.");
  return value.map(decode);
}
function stored(value: unknown): StoredStudy {
  const item = object(value);
  return {
    id: text(item.id),
    revision: number(item.revision),
    createdAt: number(item.createdAt),
    updatedAt: number(item.updatedAt),
    definition: decodeDefinition(item.definition),
    source: decodeSourceInfo(item.source),
  };
}
function summary(value: unknown): StudySummary {
  const item = object(value);
  return {
    id: text(item.id),
    title: text(item.title),
    revision: number(item.revision),
    createdAt: number(item.createdAt),
    updatedAt: number(item.updatedAt),
  };
}
function session(value: unknown): Session {
  const item = object(value);
  if (typeof item.authenticated !== "boolean" || typeof item.configured !== "boolean")
    throw new Error("Session invalide.");
  let username: string | null = null;
  let csrfToken: string | null = null;
  if (item.authenticated) {
    username = text(item.username);
    csrfToken = text(item.csrfToken);
  }
  return { authenticated: item.authenticated, configured: item.configured, username, csrfToken };
}
export class ApiError extends Error {
  readonly status: number;
  readonly issues: ValidationIssue[];
  constructor(status: number, message: string, issues: ValidationIssue[] = []) {
    super(message);
    this.status = status;
    this.issues = issues;
  }
}
export async function request(url: string, options: RequestInit = {}): Promise<unknown> {
  const response = await fetch(url, { credentials: "same-origin", cache: "no-store", ...options });
  let body: unknown;
  try {
    body = await response.json();
  } catch {
    if (response.status === 413)
      throw new ApiError(413, "Le questionnaire dépasse la taille maximale autorisée (8 Mio).");
    throw new ApiError(response.status, "Réponse du serveur illisible.");
  }
  if (!response.ok) {
    const error = object(object(body).error);
    const issues: ValidationIssue[] = [];
    if (Array.isArray(error.issues))
      for (const value of error.issues) {
        const issue = object(value);
        issues.push({ path: text(issue.path), message: text(issue.message) });
      }
    throw new ApiError(response.status, text(error.message), issues);
  }
  return body;
}
export function write(method: string, body: unknown, csrf?: string): RequestInit {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    "X-Requested-With": "RustSurvey",
  };
  if (csrf !== undefined) headers["X-CSRF-Token"] = csrf;
  return { method, headers, body: JSON.stringify(body) };
}
export async function getSession(signal?: AbortSignal): Promise<Session> {
  return session(await request("/api/admin/session", { signal: signal ?? null }));
}
export async function login(username: string, password: string): Promise<Session> {
  return session(await request("/api/admin/login", write("POST", { username, password })));
}
export async function logout(csrf: string): Promise<Session> {
  return session(await request("/api/admin/logout", write("POST", {}, csrf)));
}
export async function listStudies(
  offset: number,
  signal?: AbortSignal,
): Promise<{ items: StudySummary[]; hasMore: boolean }> {
  const body = object(await request(`/api/studies?offset=${offset}`, { signal: signal ?? null }));
  if (typeof body.hasMore !== "boolean") throw new Error("Pagination invalide.");
  return { items: array(body.items, summary), hasMore: body.hasMore };
}
export async function getStudy(id: string, signal?: AbortSignal): Promise<StoredStudy> {
  return stored(
    await request(`/api/studies/${encodeURIComponent(id)}`, { signal: signal ?? null }),
  );
}
export async function saveStudy(
  id: string | null,
  revision: number,
  definition: Definition,
  csrf: string,
  signal?: AbortSignal,
): Promise<StoredStudy> {
  if (id === null)
    return stored(
      await request("/api/admin/studies", {
        ...write("POST", definition, csrf),
        signal: signal ?? null,
      }),
    );
  return stored(
    await request(`/api/admin/studies/${encodeURIComponent(id)}`, {
      ...write("PUT", { expectedRevision: revision, definition }, csrf),
      signal: signal ?? null,
    }),
  );
}
export function message(error: unknown): string {
  if (error instanceof TypeError)
    return "Le serveur est injoignable. Réessayez après avoir vérifié sa connexion.";
  if (error instanceof Error) return error.message;
  return "L’opération a échoué.";
}

export interface RevisionSummary {
  revision: number;
  createdAt: number;
}
export interface DefinitionRevision extends RevisionSummary {
  studyId: string;
  definition: Definition;
}
export async function listRevisions(
  id: string,
  before: number,
  signal: AbortSignal,
): Promise<{ items: RevisionSummary[]; hasMore: boolean }> {
  const body = object(
    await request(`/api/admin/studies/${encodeURIComponent(id)}/revisions?before=${before}`, {
      signal,
    }),
  );
  if (typeof body.hasMore !== "boolean") throw new Error("Pagination invalide.");
  return {
    items: array(body.items, (value) => {
      const item = object(value);
      return { revision: number(item.revision), createdAt: number(item.createdAt) };
    }),
    hasMore: body.hasMore,
  };
}
export async function getRevision(
  id: string,
  revision: number,
  signal: AbortSignal,
): Promise<DefinitionRevision> {
  const item = object(
    await request(`/api/admin/studies/${encodeURIComponent(id)}/revisions/${revision}`, { signal }),
  );
  return {
    studyId: text(item.studyId),
    revision: number(item.revision),
    createdAt: number(item.createdAt),
    definition: decodeDefinition(item.definition),
  };
}
export async function restoreRevision(
  id: string,
  revision: number,
  expectedRevision: number,
  csrf: string,
  signal: AbortSignal,
): Promise<StoredStudy> {
  return stored(
    await request(`/api/admin/studies/${encodeURIComponent(id)}/restore`, {
      ...write("POST", { revision, expectedRevision }, csrf),
      signal,
    }),
  );
}

export async function importLss(
  file: File,
  csrf: string,
  signal?: AbortSignal,
): Promise<StoredStudy> {
  if (file.size === 0) throw new Error("Le fichier sélectionné est vide.");
  if (file.size > 8 * 1024 * 1024) throw new Error("Import LSS limité à 8 Mio.");
  return stored(
    await request("/api/admin/imports/lss", {
      method: "POST",
      body: file,
      signal: signal ?? null,
      headers: {
        "Content-Type": "application/xml",
        "X-Requested-With": "RustSurvey",
        "X-CSRF-Token": csrf,
      },
    }),
  );
}

export async function importDefinition(
  file: File,
  csrf: string,
  signal: AbortSignal,
): Promise<StoredStudy> {
  if (file.size === 0 || file.size > 8 * 1024 * 1024)
    throw new Error("Définition JSON non vide, limitée à 8 Mio.");
  let definition: unknown;
  try {
    definition = JSON.parse(await file.text());
  } catch {
    throw new Error("Le fichier ne contient pas un JSON valide.");
  }
  // Preserve unknown fields so the Rust decoder can reject them rather than silently discard data.
  return stored(
    await request("/api/admin/studies", { ...write("POST", definition, csrf), signal }),
  );
}

export async function downloadLss(
  id: string,
  revision: number | null,
  signal: AbortSignal,
): Promise<Blob> {
  let path = `/api/admin/studies/${encodeURIComponent(id)}/original.lss`;
  if (revision !== null)
    path = `/api/admin/studies/${encodeURIComponent(id)}/revisions/${revision}/modified.lss`;
  const response = await fetch(path, { credentials: "same-origin", cache: "no-store", signal });
  if (!response.ok) {
    let reason: string;
    try {
      reason = text(object(object(await response.json()).error).message);
    } catch {
      reason = "Le serveur a refusé le téléchargement du LSS.";
    }
    throw new ApiError(response.status, reason);
  }
  if (!response.headers.get("Content-Type")?.startsWith("application/xml"))
    throw new Error("La réponse du serveur ne contient pas un LSS.");
  return response.blob();
}
