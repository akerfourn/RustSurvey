import type { DefaultValue } from "./defaults.ts";
import type { RuntimeQuestion } from "./engine.ts";
import type { SurveyStudy } from "./types.ts";
import { variableName } from "./runtime-rules.ts";
import {
  compileDefaultTemplate,
  renderDefaultTemplate,
  type DefaultBudget,
  type DefaultTemplate,
} from "./default-template.ts";

export interface RuntimeDefaults {
  fields: Map<string, Map<string, DefaultTemplate>>;
  variables: Set<string>;
  order: string[];
  baseLanguage: string;
  shared: boolean;
}

export function prepareDefaults(study: SurveyStudy, question: RuntimeQuestion): RuntimeDefaults {
  const defaults = question.source.defaults;
  const fields: RuntimeDefaults["fields"] = new Map();
  const variables = new Set<string>();
  const baseLanguage = study.baseLanguage ?? study.languages[0] ?? "fr";
  const shared = defaults?.sameForAllLanguages ?? false;
  for (const entry of defaults?.values ?? []) {
    const key = defaultField(question, entry);
    if (!study.languages.includes(entry.language) && entry.language !== baseLanguage) continue;
    if (shared && entry.language !== baseLanguage) continue;
    const template = compileDefaultTemplate(entry.value);
    for (const name of template.variables) {
      if (name.endsWith(".NAOK"))
        throw new Error(
          "Les défauts utilisant NAOK nécessitent encore un contexte de valeurs inactives vérifié.",
        );
      variables.add(name);
    }
    const localized = fields.get(key) ?? new Map<string, DefaultTemplate>();
    if (localized.has(entry.language)) throw new Error("Valeur par défaut dupliquée.");
    localized.set(entry.language, template);
    fields.set(key, localized);
  }
  return { fields, variables, order: fieldOrder(fields), baseLanguage, shared };
}

function defaultField(question: RuntimeQuestion, entry: DefaultValue): string {
  const type = question.source.sourceType;
  const coordinate = entry.coordinate;
  const scalar =
    coordinate.kind === "primary" &&
    entry.role === "answer" &&
    ["S", "T", "U", "N", "D", "L", "!", "5", "Y", "G", "O"].includes(type);
  const other =
    coordinate.kind === "primary" &&
    entry.role === "other" &&
    ["L", "!"].includes(type) &&
    question.source.other;
  const row =
    coordinate.kind === "subquestion" &&
    coordinate.scale === null &&
    entry.role === "answer" &&
    ["M", "P", "Q", "K", "A", "B", "C", "E", "F", "H"].includes(type);
  if (!scalar && !other && !row)
    throw new Error("Cellule incompatible avec une valeur par défaut.");
  const cell = question.cells.find((cell) => {
    if (cell.role !== entry.role || cell.columnId !== null) return false;
    if (coordinate.kind === "primary") return cell.rowId === null;
    return cell.rowId === coordinate.id && cell.scale === 0;
  });
  if (cell === undefined) throw new Error("Cellule de défaut inconnue.");
  return cell.key;
}

function fieldOrder(fields: RuntimeDefaults["fields"]): string[] {
  const dependencies = new Map<string, Set<string>>();
  for (const [key, localized] of fields) {
    const required = new Set<string>();
    for (const template of localized.values()) {
      for (const variable of template.variables) {
        const name = variableName(variable);
        if (name !== key && fields.has(name)) required.add(name);
      }
    }
    dependencies.set(key, required);
  }
  const order: string[] = [];
  const done = new Set<string>();
  while (done.size < fields.size) {
    const next = [...dependencies.keys()]
      .sort()
      .find((key) => !done.has(key) && [...dependencies.get(key)!].every((name) => done.has(name)));
    if (next === undefined) throw new Error("Cycle entre valeurs par défaut d’une question.");
    order.push(next);
    done.add(next);
  }
  return order;
}

export function applyDefaults(
  defaults: RuntimeDefaults,
  answers: Map<string, string>,
  language: string,
  initialized: Set<string>,
  budget: DefaultBudget,
): void {
  let selectedLanguage = language;
  if (defaults.shared) selectedLanguage = defaults.baseLanguage;
  for (const key of defaults.order) {
    if (answers.has(key)) continue;
    const template = defaults.fields.get(key)?.get(selectedLanguage);
    if (template === undefined) continue;
    const value = renderDefaultTemplate(
      template,
      (name) => answers.get(variableName(name)) ?? "",
      budget,
    );
    answers.set(key, value);
    initialized.add(key);
  }
}

export function checkInactiveDefaultReferences(
  questions: ReadonlyMap<string, RuntimeQuestion>,
  fields: ReadonlyMap<string, string>,
): void {
  for (const question of questions.values()) {
    const expressions = [
      ...question.relevance,
      ...question.constraints.values(),
      ...question.subquestionRules.values(),
    ];
    if (question.equation !== null) expressions.push(question.equation);
    for (const expression of expressions) {
      for (const name of expression.variables) {
        const owner = questions.get(fields.get(variableName(name)) ?? "");
        if (name.endsWith(".NAOK") && (owner?.defaults?.fields.size ?? 0) > 0)
          question.issues.push(
            "Une référence NAOK à une question avec défaut nécessite encore un contexte de valeurs inactives vérifié.",
          );
      }
    }
  }
}
