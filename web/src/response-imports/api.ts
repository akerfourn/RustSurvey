import { object, request, write } from "../authoring/api.ts";

export interface ImportSummary {
  id: string;
  revision: number;
  state: "pending" | "loading" | "prepared";
  rowCount: number;
  loadedRows: number;
  sha256: string;
  createdAt: number;
}

export interface ImportReport {
  conservation: string;
  mapping: string;
  execution: string;
  continuity: string;
  submitted: number;
  drafts: number;
  unknownStatus: number;
  mappedCells: number;
  unknownColumnCount: number;
  missingColumnCount: number;
  duplicateIds: number;
  invalidIds: number;
  issueCount: number;
  issues: { path: string; message: string }[];
}

export interface ImportDetails extends ImportSummary {
  report: ImportReport;
}

export interface SourceRow {
  ordinal: number;
  sourceId: string | null;
  status: "submitted" | "draft" | "unknown";
}

function number(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw new Error("Compteur d'import invalide.");
  return value;
}

function text(value: unknown): string {
  if (typeof value !== "string") throw new Error("Texte d'import invalide.");
  return value;
}

function summary(value: unknown): ImportSummary {
  const item = object(value);
  const state = item.state;
  if (state !== "pending" && state !== "loading" && state !== "prepared")
    throw new Error("État d'import invalide.");
  const result: ImportSummary = {
    id: text(item.id),
    revision: number(item.revision),
    state,
    rowCount: number(item.rowCount),
    loadedRows: number(item.loadedRows),
    sha256: text(item.sha256),
    createdAt: number(item.createdAt),
  };
  if (
    result.loadedRows > result.rowCount ||
    result.rowCount > 100_000 ||
    (state === "prepared" && result.loadedRows !== result.rowCount)
  )
    throw new Error("Progression d'import invalide.");
  return result;
}

export function decodeDetails(value: unknown): ImportDetails {
  const item = object(value);
  const raw = object(item.report);
  if (!Array.isArray(raw.issues)) throw new Error("Diagnostics d'import invalides.");
  const issues = raw.issues.map((value: unknown) => {
    const issue = object(value);
    return { path: text(issue.path), message: text(issue.message) };
  });
  const report: ImportReport = {
    conservation: text(raw.conservation),
    mapping: text(raw.mapping),
    execution: text(raw.execution),
    continuity: text(raw.continuity),
    submitted: number(raw.submitted),
    drafts: number(raw.drafts),
    unknownStatus: number(raw.unknownStatus),
    mappedCells: number(raw.mappedCells),
    unknownColumnCount: number(raw.unknownColumnCount),
    missingColumnCount: number(raw.missingColumnCount),
    duplicateIds: number(raw.duplicateIds),
    invalidIds: number(raw.invalidIds),
    issueCount: number(raw.issueCount),
    issues,
  };
  return { ...summary(item), report };
}

export async function listImports(study: string, signal: AbortSignal): Promise<ImportSummary[]> {
  const result = object(
    await request(`/api/admin/studies/${encodeURIComponent(study)}/response-imports`, { signal }),
  );
  if (!Array.isArray(result.items)) throw new Error("Liste d'imports invalide.");
  return result.items.map(summary);
}

export async function getImport(id: string, signal: AbortSignal): Promise<ImportDetails> {
  return decodeDetails(
    await request(`/api/admin/response-imports/${encodeURIComponent(id)}`, { signal }),
  );
}

export async function upload(
  study: string,
  revision: number,
  file: File,
  csrf: string,
  signal: AbortSignal,
): Promise<ImportDetails> {
  if (file.size === 0 || file.size > 8 * 1024 * 1024)
    throw new Error("Le fichier LSR doit contenir entre 1 octet et 8 Mio.");
  return decodeDetails(
    await request(
      `/api/admin/studies/${encodeURIComponent(study)}/response-imports?revision=${revision}`,
      {
        method: "POST",
        body: file,
        signal,
        headers: {
          "Content-Type": "application/xml",
          "X-CSRF-Token": csrf,
          "X-Requested-With": "RustSurvey",
        },
      },
    ),
  );
}

export async function complete(
  batch: ImportDetails,
  csrf: string,
  signal: AbortSignal,
  onProgress: (batch: ImportDetails) => void,
): Promise<ImportDetails> {
  let current = batch;
  onProgress(current);
  while (current.state !== "prepared") {
    const previous = current.loadedRows;
    const next = decodeDetails(
      await request(`/api/admin/response-imports/${encodeURIComponent(current.id)}/continue`, {
        ...write("POST", {}, csrf),
        signal,
      }),
    );
    if (
      next.id !== batch.id ||
      next.revision !== batch.revision ||
      next.sha256 !== batch.sha256 ||
      next.rowCount !== batch.rowCount
    )
      throw new Error("L'identité de la préparation a changé.");
    current = next;
    if (current.loadedRows <= previous && current.state !== "prepared")
      throw new Error("La préparation n'a pas progressé. Actualisez son état avant de reprendre.");
    onProgress(current);
  }
  return current;
}

export async function listRows(
  id: string,
  from: number,
  signal: AbortSignal,
): Promise<{ items: SourceRow[]; hasMore: boolean }> {
  const result = object(
    await request(`/api/admin/response-imports/${encodeURIComponent(id)}/records?from=${from}`, {
      signal,
    }),
  );
  if (!Array.isArray(result.items) || typeof result.hasMore !== "boolean")
    throw new Error("Liste de lignes invalide.");
  const items = result.items.map((value: unknown): SourceRow => {
    const item = object(value);
    if (item.status !== "submitted" && item.status !== "draft" && item.status !== "unknown")
      throw new Error("État de ligne invalide.");
    let sourceId = null;
    if (item.sourceId !== null) sourceId = text(item.sourceId);
    return { ordinal: number(item.ordinal), sourceId, status: item.status };
  });
  return { items, hasMore: result.hasMore };
}
