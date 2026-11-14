import { attribute } from "./attributes.ts";
import type { ResponseCell } from "./cells.ts";
import type { RuntimeQuestion, SurveyRuntime } from "./engine.ts";
import type { Answers, SurveyQuestion, SurveySubquestion } from "./types.ts";

export interface ExclusiveOptions {
  fields: Set<string>;
  automatic: boolean;
}

export const EXCLUSIVE_TYPES = new Set(["A", "B", "C", "E", "F", "M", "P", "K", "Q"]);

export function parseExclusive(
  question: SurveyQuestion,
  rows: SurveySubquestion[],
): ExclusiveOptions {
  const source = attribute(question, "exclude_all_others");
  const auto = attribute(question, "exclude_all_others_auto");
  const seen = new Set<string>();
  for (const item of question.attributes) {
    if (["exclude_all_others", "exclude_all_others_auto"].includes(item.name)) {
      if (seen.has(item.name))
        throw new Error("Une option d’exclusivité est déclarée plusieurs fois.");
      seen.add(item.name);
    }
    if (
      ["exclude_all_others", "exclude_all_others_auto"].includes(item.name) &&
      item.language !== null &&
      item.language !== ""
    )
      throw new Error("Les options d’exclusivité ne peuvent pas être localisées.");
  }
  if (!["", "0", "1"].includes(auto))
    throw new Error("La sélection exclusive automatique doit valoir 0 ou 1.");
  const codes = new Set(
    source
      .split(";")
      .map((code) => code.trim())
      .filter((code) => code !== ""),
  );
  if (codes.size === 0 && auto !== "1") return { fields: new Set(), automatic: false };
  if (!EXCLUSIVE_TYPES.has(question.sourceType))
    throw new Error("Ce type de question ne prend pas en charge les lignes exclusives.");
  const multiple = ["M", "P"].includes(question.sourceType);
  if (auto === "1" && !multiple)
    throw new Error("La sélection exclusive automatique exige une question à choix multiples.");
  for (const code of codes) {
    const other = multiple && question.other && code === "other";
    if (!other && !rows.some((row) => row.scale === 0 && row.code === code))
      throw new Error("Un code de ligne exclusive est inconnu.");
  }
  return {
    fields: new Set([...codes].map((code) => `${question.code}_${code}`)),
    automatic: auto === "1" && codes.size === 1 && !source.trim().includes(";"),
  };
}

function selection(cell: ResponseCell): string {
  if (cell.role === "comment") return cell.key.replace(/comment$/, "");
  return cell.key;
}

export function applyExclusive(
  question: RuntimeQuestion,
  answers: Map<string, string>,
  active: Set<string>,
): void {
  const options = question.exclusive;
  if (options.fields.size === 0) return;
  const primary = question.cells.filter((cell) => cell.role !== "comment");
  const selected = primary.filter(
    (cell) =>
      active.has(cell.key) && options.fields.has(cell.key) && (answers.get(cell.key) ?? "") !== "",
  );
  if (selected.length > 1)
    throw new Error("Plusieurs lignes exclusives sont renseignées simultanément.");
  let chosen = selected[0]?.key;
  let automatic = false;
  if (chosen === undefined && options.automatic) {
    const others = primary.filter((cell) => !options.fields.has(cell.key));
    const allAnswered =
      others.length > 0 &&
      others.every((cell) => !active.has(cell.key) || (answers.get(cell.key) ?? "") !== "");
    if (allAnswered) {
      chosen = primary.find((cell) => active.has(cell.key) && options.fields.has(cell.key))?.key;
      if (chosen !== undefined) {
        answers.set(chosen, "Y");
        automatic = true;
      }
    }
  }
  if (chosen === undefined) return;
  for (const cell of question.cells) {
    if (selection(cell) !== chosen) {
      active.delete(cell.key);
      answers.delete(cell.key);
    } else if (automatic && cell.role === "comment") answers.delete(cell.key);
  }
}

export function preserveExistingExclusiveSelection(
  question: RuntimeQuestion,
  answers: Map<string, string>,
  active: ReadonlySet<string>,
  initialized: ReadonlySet<string>,
): void {
  if (initialized.size === 0 || question.exclusive.fields.size === 0) return;
  const selected = question.cells.some(
    (cell) =>
      cell.role !== "comment" &&
      question.exclusive.fields.has(cell.key) &&
      active.has(cell.key) &&
      !initialized.has(cell.key) &&
      (answers.get(cell.key) ?? "") !== "",
  );
  if (!selected) return;
  for (const key of question.exclusive.fields) {
    if (initialized.has(key)) answers.delete(key);
  }
}

export function applyComments(
  question: RuntimeQuestion,
  answers: Map<string, string>,
  active: Set<string>,
): void {
  if (question.source.sourceType !== "P") return;
  const mode = attribute(question.source, "commented_checkbox") || "checked";
  for (const cell of question.cells) {
    if (cell.role !== "comment") continue;
    const selected = (answers.get(selection(cell)) ?? "") !== "";
    if ((mode === "checked" && !selected) || (mode === "unchecked" && selected)) {
      active.delete(cell.key);
      answers.delete(cell.key);
    }
  }
}

export function changeExclusiveAnswers(
  runtime: SurveyRuntime,
  previous: Answers,
  changes: readonly (readonly [string, string])[],
): Answers {
  const updated = new Map(previous);
  for (const [field, value] of changes) {
    const question = runtime.questions.get(runtime.fields.get(field) ?? "");
    if (question !== undefined && value !== "" && question.exclusive.fields.has(field)) {
      for (const cell of question.cells) if (selection(cell) !== field) updated.delete(cell.key);
    }
    updated.set(field, value);
  }
  return updated;
}
