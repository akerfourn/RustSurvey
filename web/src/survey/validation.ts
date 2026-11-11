import {
  answerLookup,
  attribute,
  errorMessage,
  type RuntimeQuestion,
  type Snapshot,
  type SurveyRuntime,
} from "./engine.ts";
import { numeric, truth } from "./expressions.ts";
import type { Answers } from "./types.ts";

export interface Validation {
  errors: Map<string, string[]>;
  warnings: Map<string, string[]>;
}

function allowedValues(question: RuntimeQuestion): Set<string> | null {
  const type = question.source.sourceType;
  if (["L", "!", "F", "H"].includes(type))
    return new Set(question.choices.map((choice) => choice.code));
  if (type === "5") return new Set(["1", "2", "3", "4", "5"]);
  if (type === "Y") return new Set(["Y", "N"]);
  if (type === "G") return new Set(["M", "F"]);
  if (type === "M") return new Set(["Y"]);
  if (type === ":") return new Set(["0", "1"]);
  return null;
}

function isDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const date = new Date(`${value}T00:00:00Z`);
  return Number.isFinite(date.getTime()) && date.toISOString().slice(0, 10) === value;
}

function validateValues(question: RuntimeQuestion, answers: Answers): string[] {
  const errors: string[] = [];
  const allowed = allowedValues(question);
  for (const cell of question.cells) {
    const value = answers.get(cell.key) ?? "";
    if (value === "") continue;
    if (allowed !== null && !allowed.has(value)) errors.push("Choix de réponse inconnu.");
    if (cell.kind === "number") {
      try {
        const number = numeric(value);
        if (attribute(question.source, "num_value_int_only") === "1" && !Number.isInteger(number)) {
          errors.push("Un nombre entier est attendu.");
        }
      } catch {
        errors.push("Un nombre valide est attendu.");
      }
    }
    if (cell.kind === "date" && !isDate(value)) errors.push("Une date valide est attendue.");
  }
  return errors;
}

function validateConstraints(question: RuntimeQuestion, answers: Answers): string[] {
  const errors: string[] = [];
  const values = question.fields
    .map((field) => answers.get(field) ?? "")
    .filter((value) => value !== "");
  let answerCount = values.length;
  if (question.source.sourceType === ":")
    answerCount = values.filter((value) => value === "1").length;
  for (const [name, expression] of question.constraints) {
    try {
      const result = expression.evaluate(answerLookup(answers));
      if (name === "em_validation_q") {
        if (!truth(result))
          errors.push("La règle de validation de cette question n’est pas satisfaite.");
        continue;
      }
      const limit = numeric(result);
      if (name === "min_answers" && answerCount < limit)
        errors.push(`Sélectionnez au moins ${limit} réponse(s).`);
      if (name === "max_answers" && answerCount > limit)
        errors.push(`Sélectionnez au plus ${limit} réponse(s).`);
      for (const value of values) {
        if (name === "maximum_chars" && [...value].length > limit)
          errors.push(`La réponse est limitée à ${limit} caractères.`);
        if (name === "min_num_value_n" && numeric(value) < limit)
          errors.push(`La valeur minimale est ${limit}.`);
        if (name === "max_num_value_n" && numeric(value) > limit)
          errors.push(`La valeur maximale est ${limit}.`);
      }
    } catch (error) {
      errors.push(errorMessage(error));
    }
  }
  return errors;
}

export function validate(runtime: SurveyRuntime, snapshot: Snapshot, groupId?: string): Validation {
  const errors = new Map<string, string[]>();
  const warnings = new Map<string, string[]>();
  for (const [id, question] of runtime.questions) {
    if (
      !snapshot.visible.has(id) ||
      question.source.sourceType === "X" ||
      question.issues.length > 0
    )
      continue;
    if (groupId !== undefined && question.source.groupId !== groupId) continue;
    const missing = question.fields.filter((field) => (snapshot.answers.get(field) ?? "") === "");
    let unanswered = missing.length > 0;
    if (question.source.sourceType === "M") unanswered = missing.length === question.fields.length;
    if (question.source.sourceType === ":") {
      const rows = new Set(question.cells.map((cell) => cell.rowId));
      const checkedRows = new Set(
        question.cells
          .filter((cell) => snapshot.answers.get(cell.key) === "1")
          .map((cell) => cell.rowId),
      );
      unanswered = [...rows].some((row) => !checkedRows.has(row));
    }
    const questionErrors = validateValues(question, snapshot.answers);
    if (unanswered && question.source.mandatory === "strict")
      questionErrors.push("Cette question est obligatoire.");
    if (unanswered && question.source.mandatory === "soft")
      warnings.set(id, ["Il est recommandé de répondre à cette question."]);
    questionErrors.push(...validateConstraints(question, snapshot.answers));
    if (questionErrors.length > 0) errors.set(id, [...new Set(questionErrors)]);
  }
  return { errors, warnings };
}
