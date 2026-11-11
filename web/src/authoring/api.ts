import {
  QUESTION_KINDS,
  type Definition,
  type Group,
  type Question,
  type Session,
  type StoredStudy,
  type StudySummary,
  type ValidationIssue,
  type Choice,
  type QuestionKind,
  type Mandatory,
} from "./model.ts";

function object(value: unknown): Record<string, unknown> {
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
function choice(value: unknown): Choice {
  const item = object(value);
  return { id: text(item.id), code: text(item.code), label: text(item.label) };
}
function question(value: unknown): Question {
  const item = object(value);
  const kind = text(item.kind);
  if (!QUESTION_KINDS.has(kind as QuestionKind)) throw new Error("Type de question inconnu.");
  const mandatory = text(item.mandatory);
  if (!["no", "soft", "strict"].includes(mandatory))
    throw new Error("Caractère obligatoire inconnu.");
  return {
    id: text(item.id),
    code: text(item.code),
    title: text(item.title),
    help: text(item.help),
    kind: kind as QuestionKind,
    mandatory: mandatory as Mandatory,
    choices: array(item.choices, choice),
  };
}
function group(value: unknown): Group {
  const item = object(value);
  return {
    id: text(item.id),
    title: text(item.title),
    description: text(item.description),
    questions: array(item.questions, question),
  };
}
export function decodeDefinition(value: unknown): Definition {
  const item = object(value);
  if (item.schemaVersion !== 1) throw new Error("Version de questionnaire non prise en charge.");
  return {
    schemaVersion: 1,
    title: text(item.title),
    language: text(item.language),
    welcome: text(item.welcome),
    end: text(item.end),
    groups: array(item.groups, group),
  };
}
function stored(value: unknown): StoredStudy {
  const item = object(value);
  return {
    id: text(item.id),
    revision: number(item.revision),
    createdAt: number(item.createdAt),
    updatedAt: number(item.updatedAt),
    definition: decodeDefinition(item.definition),
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
async function request(url: string, options: RequestInit = {}): Promise<unknown> {
  const response = await fetch(url, { credentials: "same-origin", cache: "no-store", ...options });
  let body: unknown;
  try {
    body = await response.json();
  } catch {
    if (response.status === 413)
      throw new ApiError(413, "Le questionnaire dépasse la taille maximale autorisée (1 Mio).");
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
function write(method: string, body: unknown, csrf?: string): RequestInit {
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
