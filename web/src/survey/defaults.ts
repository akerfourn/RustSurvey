export interface PrimaryDefaultCoordinate {
  kind: "primary";
}
export interface SubquestionDefaultCoordinate {
  kind: "subquestion";
  id: string;
  scale: null;
}
export type DefaultCoordinate = PrimaryDefaultCoordinate | SubquestionDefaultCoordinate;
export interface DefaultValue {
  coordinate: DefaultCoordinate;
  role: "answer" | "other";
  language: string;
  value: string;
}
export interface QuestionDefaults {
  sameForAllLanguages: boolean;
  values: DefaultValue[];
}

function record(value: unknown, keys: readonly string[]): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value))
    throw new Error("Valeurs par défaut invalides.");
  const result = value as Record<string, unknown>;
  if (Object.keys(result).some((key) => !keys.includes(key)))
    throw new Error("Champ de valeur par défaut inconnu.");
  return result;
}

function coordinate(value: unknown): DefaultCoordinate {
  const item = record(value, ["kind", "id", "scale"]);
  if (item.kind === "primary" && item.id === undefined && item.scale === undefined)
    return { kind: "primary" };
  if (
    item.kind === "subquestion" &&
    typeof item.id === "string" &&
    item.id !== "" &&
    (item.scale === null || item.scale === undefined)
  )
    return { kind: "subquestion", id: item.id, scale: null };
  throw new Error("Coordonnée de valeur par défaut non prise en charge.");
}

export function decodeQuestionDefaults(value: unknown): QuestionDefaults | null {
  if (value === undefined || value === null) return null;
  const item = record(value, ["sameForAllLanguages", "values"]);
  if (
    typeof item.sameForAllLanguages !== "boolean" ||
    !Array.isArray(item.values) ||
    item.values.length > 6400
  )
    throw new Error("Configuration de valeurs par défaut invalide.");
  const values = item.values.map((value): DefaultValue => {
    const entry = record(value, ["coordinate", "role", "language", "value"]);
    if (
      (entry.role !== "answer" && entry.role !== "other") ||
      typeof entry.language !== "string" ||
      entry.language.length > 32 ||
      !/^[A-Za-z]+(?:-[A-Za-z]+)*$/.test(entry.language) ||
      typeof entry.value !== "string" ||
      entry.value.length > 4096 ||
      entry.value.includes("\0")
    )
      throw new Error("Langue, rôle ou texte de valeur par défaut invalide.");
    return {
      coordinate: coordinate(entry.coordinate),
      role: entry.role,
      language: entry.language,
      value: entry.value,
    };
  });
  return { sameForAllLanguages: item.sameForAllLanguages, values };
}

export function defaultTargetKey(
  coordinate: DefaultCoordinate,
  role: DefaultValue["role"],
): string {
  if (coordinate.kind === "primary") return role;
  return `${role}:${coordinate.id}`;
}
