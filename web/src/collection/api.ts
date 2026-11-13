import { ApiError, object, request, write } from "../authoring/api.ts";
import { decodeDefinition } from "../authoring/definition-codec.ts";
import type { Definition, ValidationIssue } from "../authoring/model.ts";

export interface Run {
  id: string;
  studyId: string;
  revision: number;
  profile: string;
  isOpen: boolean;
  createdAt: number;
  participationPolicy: ParticipationPolicy;
}

export type ParticipationPolicy =
  | { mode: "open" }
  | { mode: "browserOnce" }
  | { mode: "invitation"; persistence: boolean; allowEditAfterSubmit: boolean };

export interface Checkpoint {
  language: string;
  groupId: string | null;
}

export interface RunDefinition {
  run: Run;
  definition: Definition;
}

export interface RunEntry extends RunDefinition {
  browserCompleted: boolean;
}

export interface Participation extends RunDefinition {
  id: string;
  version: number;
  status: "draft" | "submitted";
  answers: Record<string, string>;
  checkpoint: Checkpoint;
  createdAt: number;
  updatedAt: number;
}

export interface SaveCommand {
  expectedVersion: number;
  operationId: string;
  answers: Record<string, string>;
  checkpoint: Checkpoint;
  submit: boolean;
}

export interface Acknowledgement {
  id: string;
  version: number;
  status: "draft" | "submitted";
  checkpoint: Checkpoint;
  updatedAt: number;
  warnings: ValidationIssue[];
}

function text(value: unknown): string {
  if (typeof value !== "string") throw new Error("Réponse de collecte invalide.");
  return value;
}
function integer(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value))
    throw new Error("Version de collecte invalide.");
  return value;
}
function status(value: unknown): "draft" | "submitted" {
  if (value !== "draft" && value !== "submitted") throw new Error("État de réponse invalide.");
  return value;
}
function checkpoint(value: unknown): Checkpoint {
  const item = object(value);
  let groupId: string | null = null;
  if (item.groupId !== null) groupId = text(item.groupId);
  return { language: text(item.language), groupId };
}
function policy(value: unknown): ParticipationPolicy {
  const item = object(value);
  if (item.mode === "open" || item.mode === "browserOnce") return { mode: item.mode };
  if (
    item.mode === "invitation" &&
    typeof item.persistence === "boolean" &&
    typeof item.allowEditAfterSubmit === "boolean" &&
    (item.persistence || !item.allowEditAfterSubmit)
  )
    return {
      mode: item.mode,
      persistence: item.persistence,
      allowEditAfterSubmit: item.allowEditAfterSubmit,
    };
  throw new Error("Politique de participation invalide.");
}
function run(value: unknown): Run {
  const item = object(value);
  if (typeof item.isOpen !== "boolean") throw new Error("État de campagne invalide.");
  return {
    id: text(item.id),
    studyId: text(item.studyId),
    revision: integer(item.revision),
    profile: text(item.profile),
    isOpen: item.isOpen,
    createdAt: integer(item.createdAt),
    participationPolicy: policy(item.participationPolicy),
  };
}
function participation(value: unknown): Participation {
  const item = object(value);
  const answers = Object.fromEntries(
    Object.entries(object(item.answers)).map(([key, value]) => [key, text(value)]),
  );
  return {
    id: text(item.id),
    run: run(item.run),
    definition: decodeDefinition(item.definition),
    version: integer(item.version),
    status: status(item.status),
    answers,
    checkpoint: checkpoint(item.checkpoint),
    createdAt: integer(item.createdAt),
    updatedAt: integer(item.updatedAt),
  };
}
function authorization(token: string, invitationToken?: string): Record<string, string> {
  const headers: Record<string, string> = { Authorization: `Bearer ${token}` };
  if (invitationToken !== undefined) headers["X-Participation-Token"] = invitationToken;
  return headers;
}
async function collectionRequest(url: string, options: RequestInit = {}): Promise<unknown> {
  const timeout = AbortSignal.timeout(20_000);
  let signal = timeout;
  if (options.signal !== undefined && options.signal !== null)
    signal = AbortSignal.any([options.signal, timeout]);
  try {
    return await request(url, { ...options, signal });
  } catch (error) {
    if (error instanceof ApiError && error.status === 413)
      throw new ApiError(413, "Les réponses dépassent la taille autorisée pour un enregistrement.");
    throw error;
  }
}

export async function getRun(id: string, signal?: AbortSignal): Promise<RunEntry> {
  const item = object(
    await collectionRequest(`/api/runs/${encodeURIComponent(id)}`, { signal: signal ?? null }),
  );
  if (typeof item.browserCompleted !== "boolean")
    throw new Error("État de participation invalide.");
  return {
    run: run(item.run),
    definition: decodeDefinition(item.definition),
    browserCompleted: item.browserCompleted,
  };
}
export async function getParticipation(
  id: string,
  token: string,
  signal?: AbortSignal,
): Promise<Participation> {
  return participation(
    await collectionRequest(`/api/responses/${encodeURIComponent(id)}`, {
      headers: authorization(token),
      signal: signal ?? null,
    }),
  );
}
export async function startParticipation(
  runId: string,
  responseId: string,
  token: string,
  language: string,
  signal?: AbortSignal,
  invitationToken?: string,
): Promise<Participation> {
  const options = write("POST", { responseId, language });
  return participation(
    await collectionRequest(`/api/runs/${encodeURIComponent(runId)}/responses`, {
      ...options,
      headers: { ...options.headers, ...authorization(token, invitationToken) },
      signal: signal ?? null,
    }),
  );
}
export async function saveParticipation(
  id: string,
  token: string,
  command: SaveCommand,
  signal?: AbortSignal,
  invitationToken?: string,
): Promise<Acknowledgement> {
  const options = write("PUT", command);
  const item = object(
    await collectionRequest(`/api/responses/${encodeURIComponent(id)}`, {
      ...options,
      headers: { ...options.headers, ...authorization(token, invitationToken) },
      signal: signal ?? null,
    }),
  );
  if (!Array.isArray(item.warnings)) throw new Error("Validation de collecte invalide.");
  const warnings = item.warnings.map((value) => {
    const issue = object(value);
    return { path: text(issue.path), message: text(issue.message) };
  });
  return {
    id: text(item.id),
    version: integer(item.version),
    status: status(item.status),
    checkpoint: checkpoint(item.checkpoint),
    updatedAt: integer(item.updatedAt),
    warnings,
  };
}
export function resumeFragment(id: string, token: string, invitationToken?: string): string {
  let fragment = `#/responses/${encodeURIComponent(id)}/${encodeURIComponent(token)}`;
  if (invitationToken !== undefined) fragment += `/invite/${encodeURIComponent(invitationToken)}`;
  return fragment;
}
export function invitationFragment(runId: string, token: string): string {
  return `#/runs/${encodeURIComponent(runId)}/invite/${encodeURIComponent(token)}`;
}
export function credentials(): { id: string; token: string } {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  const token = [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
  return { id: crypto.randomUUID(), token };
}

export async function listRuns(id: string, signal?: AbortSignal): Promise<Run[]> {
  const body = object(
    await request(`/api/admin/studies/${encodeURIComponent(id)}/runs`, { signal: signal ?? null }),
  );
  if (!Array.isArray(body.items)) throw new Error("Liste de campagnes invalide.");
  return body.items.map(run);
}
export async function createRun(
  id: string,
  revision: number,
  csrf: string,
  participationPolicy: ParticipationPolicy = { mode: "open" },
): Promise<Run> {
  return run(
    await request(
      `/api/admin/studies/${encodeURIComponent(id)}/runs`,
      write("POST", { expectedRevision: revision, participationPolicy }, csrf),
    ),
  );
}
export async function closeRun(id: string, csrf: string): Promise<Run> {
  return run(
    await request(`/api/admin/runs/${encodeURIComponent(id)}/close`, write("POST", {}, csrf)),
  );
}

export interface ResponseSummary {
  id: string;
  version: number;
  status: "draft" | "submitted";
  createdAt: number;
  updatedAt: number;
}
export async function listResponses(
  id: string,
  offset: number,
  signal?: AbortSignal,
): Promise<{ items: ResponseSummary[]; hasMore: boolean }> {
  const body = object(
    await request(`/api/admin/runs/${encodeURIComponent(id)}/responses?offset=${offset}`, {
      signal: signal ?? null,
    }),
  );
  if (!Array.isArray(body.items) || typeof body.hasMore !== "boolean")
    throw new Error("Liste de participations invalide.");
  const items = body.items.map((value) => {
    const item = object(value);
    return {
      id: text(item.id),
      version: integer(item.version),
      status: status(item.status),
      createdAt: integer(item.createdAt),
      updatedAt: integer(item.updatedAt),
    };
  });
  return { items, hasMore: body.hasMore };
}
