import { object, request } from "./api.ts";

export interface MediaRecord {
  id: string;
  studyId: string;
  filename: string;
  contentType: string;
  size: number;
  sha256: string;
  state: "pending" | "ready";
  createdAt: number;
}
function decode(value: unknown): MediaRecord {
  const item = object(value);
  if (
    typeof item.id !== "string" ||
    typeof item.studyId !== "string" ||
    typeof item.filename !== "string" ||
    typeof item.contentType !== "string" ||
    typeof item.sha256 !== "string" ||
    typeof item.size !== "number" ||
    !Number.isSafeInteger(item.size) ||
    typeof item.createdAt !== "number" ||
    !Number.isSafeInteger(item.createdAt) ||
    (item.state !== "pending" && item.state !== "ready")
  )
    throw new Error("Média du serveur invalide.");
  return {
    id: item.id,
    studyId: item.studyId,
    filename: item.filename,
    contentType: item.contentType,
    size: item.size,
    sha256: item.sha256,
    state: item.state,
    createdAt: item.createdAt,
  };
}
export async function listMedia(
  study: string,
  offset: number,
  signal: AbortSignal,
): Promise<{ items: MediaRecord[]; hasMore: boolean }> {
  const body = object(
    await request(`/api/admin/studies/${encodeURIComponent(study)}/media?offset=${offset}`, {
      signal,
    }),
  );
  if (!Array.isArray(body.items) || typeof body.hasMore !== "boolean")
    throw new Error("Liste de médias invalide.");
  return { items: body.items.map(decode), hasMore: body.hasMore };
}
export async function uploadMedia(
  study: string,
  id: string,
  file: File,
  csrf: string,
  signal: AbortSignal,
): Promise<MediaRecord> {
  if (file.size > 8 * 1024 * 1024) throw new Error("Image limitée à 8 Mio.");
  if (file.size === 0) throw new Error("Le fichier est vide.");
  return decode(
    await request(
      `/api/admin/studies/${encodeURIComponent(study)}/media/${encodeURIComponent(id)}?name=${encodeURIComponent(file.name)}`,
      {
        method: "PUT",
        body: file,
        signal,
        headers: {
          "Content-Type": "application/octet-stream",
          "X-Requested-With": "RustSurvey",
          "X-CSRF-Token": csrf,
        },
      },
    ),
  );
}
