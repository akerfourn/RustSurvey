import { object, request } from "./api.ts";
import type { ValidationIssue } from "./model.ts";
import { decodeGroupSettings, type GroupSetting } from "./group-settings.ts";
import { decodeSourceContext, type SourceContext } from "./source-context-api.ts";

export interface CollectionCheck {
  revision: number;
  runtime: "supported" | "blocked";
  mapping: "complete" | "partial";
  mappedCells: number;
  unprojectedSections: number;
  unprojectedColumns: number;
  missingSettings: number;
  issueCount: number;
  issues: ValidationIssue[];
  groupSettings: GroupSetting[];
  sourceContext: SourceContext | null;
}

function count(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw new Error("Compteur du bilan de collecte invalide.");
  return value;
}

export async function checkCollection(
  id: string,
  revision: number,
  signal?: AbortSignal,
  contextId: string | null = null,
): Promise<CollectionCheck> {
  let query = "";
  if (contextId !== null) query = `?contextId=${encodeURIComponent(contextId)}`;
  const result = object(
    await request(
      `/api/admin/studies/${encodeURIComponent(id)}/revisions/${revision}/collection-check${query}`,
      { signal: signal ?? null },
    ),
  );
  const projection = object(result.projection);
  let sourceContext: SourceContext | null = null;
  if (result.sourceContext !== null) sourceContext = decodeSourceContext(result.sourceContext);
  if (
    (sourceContext?.id ?? null) !== contextId ||
    (sourceContext !== null &&
      (sourceContext.studyId !== id || sourceContext.revision !== revision))
  )
    throw new Error("Le bilan utilise un autre contexte source.");
  if (
    result.schemaVersion !== 1 ||
    result.revision !== revision ||
    result.execution !== "blocked" ||
    result.continuity !== "unavailable"
  )
    throw new Error("Version ou portée du bilan de collecte invalide.");
  if (result.runtime !== "supported" && result.runtime !== "blocked")
    throw new Error("État du moteur invalide.");
  if (result.mapping !== "complete" && result.mapping !== "partial")
    throw new Error("État des correspondances invalide.");
  if (!Array.isArray(result.issues) || result.issues.length > 128)
    throw new Error("Diagnostics de collecte invalides.");
  const issues = result.issues.map((value): ValidationIssue => {
    const issue = object(value);
    if (typeof issue.path !== "string" || typeof issue.message !== "string")
      throw new Error("Diagnostic de collecte invalide.");
    return { path: issue.path, message: issue.message };
  });
  const issueCount = count(result.issueCount);
  if (issueCount < issues.length) throw new Error("Décompte des diagnostics invalide.");
  return {
    revision,
    runtime: result.runtime,
    mapping: result.mapping,
    mappedCells: count(result.mappedCells),
    unprojectedSections: count(projection.unprojectedSections),
    unprojectedColumns: count(projection.unprojectedColumns),
    missingSettings: count(projection.missingSettings),
    issueCount,
    issues,
    groupSettings: decodeGroupSettings(result.groupSettings),
    sourceContext,
  };
}
