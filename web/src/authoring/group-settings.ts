import { object } from "./api.ts";

export interface GroupSetting {
  name: string;
  status: "resolved" | "unresolved" | "invalid";
  value: boolean | number | string | null;
  origin: string;
}

export function decodeGroupSettings(value: unknown): GroupSetting[] {
  const report = object(value);
  if (
    report.schemaVersion !== 1 ||
    typeof report.complete !== "boolean" ||
    !Array.isArray(report.settings) ||
    report.settings.length > 30
  )
    throw new Error("Bilan des réglages hérités invalide.");
  const names = new Set<string>();
  const settings = report.settings.map((value): GroupSetting => {
    const item = object(value);
    if (typeof item.name !== "string" || !/^[a-z_]{1,40}$/.test(item.name) || names.has(item.name))
      throw new Error("Nom de réglage hérité invalide.");
    names.add(item.name);
    if (item.status !== "resolved" && item.status !== "unresolved" && item.status !== "invalid")
      throw new Error("État de réglage hérité invalide.");
    if (
      item.value !== null &&
      typeof item.value !== "boolean" &&
      typeof item.value !== "string" &&
      typeof item.value !== "number"
    )
      throw new Error("Valeur de réglage hérité invalide.");
    if (typeof item.value === "number" && (!Number.isSafeInteger(item.value) || item.value < 0))
      throw new Error("Nombre de réglage hérité invalide.");
    if ((item.status === "resolved") !== (item.value !== null))
      throw new Error("Résolution de réglage hérité invalide.");
    let origin = "Contexte source nécessaire";
    if (item.origin !== null) {
      const source = object(item.origin);
      if (source.layer === "survey" && source.groupId === null) origin = "Questionnaire";
      else if (
        (source.layer === "group" || source.layer === "global") &&
        typeof source.groupId === "string" &&
        /^\d{1,20}$/.test(source.groupId)
      )
        origin = `Groupe source ${source.groupId}`;
      else throw new Error("Origine de réglage hérité invalide.");
    }
    return { name: item.name, status: item.status, value: item.value, origin };
  });
  if (report.complete !== settings.every((setting) => setting.status === "resolved"))
    throw new Error("Complétude des réglages hérités invalide.");
  return settings;
}
