import { object } from "../authoring/api.ts";

export interface SourceRuntimeFacts {
  schemaVersion: 1;
  surveyId: string;
  hasParticipantsTable: boolean;
  deleteNonValues: boolean;
  showNoAnswer: boolean;
  effectiveTheme: string;
  activePlugins: number;
  timeZone: string;
  timeAdjust: string;
}

function flag(value: unknown): boolean {
  if (typeof value !== "boolean") throw new Error("Indicateur d’exécution source invalide.");
  return value;
}

export function decodeRuntimeFacts(value: unknown): SourceRuntimeFacts {
  const item = object(value);
  const allowed = new Set([
    "schemaVersion",
    "surveyId",
    "hasParticipantsTable",
    "deleteNonValues",
    "showNoAnswer",
    "effectiveTheme",
    "activePlugins",
    "timeZone",
    "timeAdjust",
  ]);
  if (Object.keys(item).some((key) => !allowed.has(key)) || item.schemaVersion !== 1)
    throw new Error("Format des paramètres d’exécution source inconnu.");
  const { surveyId, effectiveTheme, activePlugins, timeZone, timeAdjust } = item;
  if (
    typeof surveyId !== "string" ||
    !/^[1-9][0-9]{0,19}$/.test(surveyId) ||
    BigInt(surveyId) > 18446744073709551615n
  )
    throw new Error("Identité d’étude source invalide.");
  if (
    typeof effectiveTheme !== "string" ||
    effectiveTheme.length === 0 ||
    effectiveTheme.length > 128 ||
    effectiveTheme.includes("\0")
  )
    throw new Error("Nom du thème source invalide.");
  if (
    typeof activePlugins !== "number" ||
    !Number.isSafeInteger(activePlugins) ||
    activePlugins < 0 ||
    activePlugins > 4294967295
  )
    throw new Error("Nombre d’extensions source invalide.");
  return {
    timeZone: clockText(timeZone),
    timeAdjust: clockText(timeAdjust),
    schemaVersion: 1,
    surveyId,
    effectiveTheme,
    activePlugins,
    hasParticipantsTable: flag(item.hasParticipantsTable),
    deleteNonValues: flag(item.deleteNonValues),
    showNoAnswer: flag(item.showNoAnswer),
  };
}

function clockText(value: unknown): string {
  if (typeof value !== "string" || value.length === 0 || value.length > 64 || value.includes("\0"))
    throw new Error("Contexte horaire source invalide.");
  return value;
}

export async function readRuntimeFacts(file: File): Promise<SourceRuntimeFacts> {
  if (file.size === 0 || file.size > 65536)
    throw new Error("Choisissez un fichier de paramètres d’exécution de 1 octet à 64 Kio.");
  const text = await file.text();
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new Error("Le fichier des paramètres d’exécution n’est pas un JSON lisible.");
  }
  return decodeRuntimeFacts(value);
}
