import { object, request } from "./api.ts";

export interface SourceContext {
  id: string;
  studyId: string;
  revision: number;
  sourceSha256: string;
  sha256: string;
  size: number;
  profile: string;
  complete: boolean;
  createdAt: number;
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export function decodeSourceContext(value: unknown): SourceContext {
  const item = object(value);
  for (const field of ["id", "studyId"])
    if (typeof item[field] !== "string" || !UUID.test(item[field]))
      throw new Error("Identité de contexte source invalide.");
  for (const field of ["sha256", "sourceSha256"])
    if (typeof item[field] !== "string" || !/^[a-f0-9]{64}$/.test(item[field]))
      throw new Error("Empreinte de contexte source invalide.");
  for (const field of ["revision", "size", "createdAt"])
    if (typeof item[field] !== "number" || !Number.isSafeInteger(item[field]))
      throw new Error("Métadonnée de contexte source invalide.");
  if (
    (item.revision as number) < 1 ||
    (item.size as number) < 1 ||
    (item.size as number) > 65536 ||
    typeof item.complete !== "boolean" ||
    typeof item.profile !== "string" ||
    !/^[a-z0-9-]{1,64}$/.test(item.profile)
  )
    throw new Error("Portée de contexte source invalide.");
  return {
    id: item.id as string,
    studyId: item.studyId as string,
    revision: item.revision as number,
    sourceSha256: item.sourceSha256 as string,
    sha256: item.sha256 as string,
    size: item.size as number,
    profile: item.profile,
    complete: item.complete,
    createdAt: item.createdAt as number,
  };
}

function inScope(context: SourceContext, studyId: string, revision: number): SourceContext {
  if (context.studyId !== studyId || context.revision !== revision)
    throw new Error("Le contexte appartient à une autre révision.");
  return context;
}

export async function listSourceContexts(
  studyId: string,
  revision: number,
  signal?: AbortSignal,
): Promise<SourceContext[]> {
  const result = object(
    await request(
      `/api/admin/studies/${encodeURIComponent(studyId)}/revisions/${revision}/settings-contexts`,
      { signal: signal ?? null },
    ),
  );
  if (!Array.isArray(result.items) || result.items.length > 32)
    throw new Error("Liste de contextes source invalide.");
  const items = result.items.map((item) => inScope(decodeSourceContext(item), studyId, revision));
  if (new Set(items.map((item) => item.id)).size !== items.length)
    throw new Error("Contextes source dupliqués.");
  return items;
}

export async function uploadSourceContext(
  studyId: string,
  revision: number,
  file: File,
  csrf: string,
  signal?: AbortSignal,
): Promise<SourceContext> {
  if (file.size === 0 || file.size > 65536)
    throw new Error("Choisissez un contexte JSON de 1 octet à 64 Kio.");
  const result = await request(
    `/api/admin/studies/${encodeURIComponent(studyId)}/revisions/${revision}/settings-contexts`,
    {
      method: "POST",
      body: file,
      signal: signal ?? null,
      headers: {
        "Content-Type": "application/json",
        "X-Requested-With": "RustSurvey",
        "X-CSRF-Token": csrf,
      },
    },
  );
  return inScope(decodeSourceContext(result), studyId, revision);
}
