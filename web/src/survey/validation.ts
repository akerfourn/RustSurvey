import {
  answerLookup,
  attribute,
  errorMessage,
  type RuntimeQuestion,
  type Snapshot,
  type SurveyRuntime,
} from "./engine.ts";
import { isCheckboxMatrix } from "./attributes.ts";
import { choiceCodes } from "./question-kinds.ts";
import { matrixInputBounds, matrixRange } from "./matrix-options.ts";
import { numeric, truth } from "./expressions.ts";
import type { ResponseCell } from "./cells.ts";
import type { Answers } from "./types.ts";

export interface Validation {
  errors: Map<string, string[]>;
  warnings: Map<string, string[]>;
}

function isDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const date = new Date(`${value}T00:00:00Z`);
  return Number.isFinite(date.getTime()) && date.toISOString().slice(0, 10) === value;
}

function validateValues(
  question: RuntimeQuestion,
  cells: ResponseCell[],
  answers: Answers,
  languages: string[],
): string[] {
  const errors: string[] = [];
  for (const cell of cells) {
    const value = answers.get(cell.key) ?? "";
    if (value === "") continue;
    if (cell.kind === "choice") {
      let allowed = choiceCodes(question.source.sourceType, question.choices, cell.scale);
      if (question.source.sourceType === "I") allowed = languages;
      if (
        question.source.other &&
        cell.role === "answer" &&
        ["L", "!"].includes(question.source.sourceType)
      )
        allowed.push("-oth-");
      if (!allowed.includes(value)) errors.push("Choix de réponse inconnu.");
    }
    if (cell.kind === "multiple" && value !== "Y") errors.push("Choix de réponse inconnu.");
    if (cell.kind === "checkbox" && !["0", "1"].includes(value))
      errors.push("Choix de réponse inconnu.");
    if (
      cell.kind === "number" ||
      (cell.role === "other" && attribute(question.source, "other_numbers_only") === "1")
    ) {
      try {
        const number = numeric(value);
        if (attribute(question.source, "num_value_int_only") === "1" && !Number.isInteger(number))
          errors.push("Un nombre entier est attendu.");
        if (
          question.source.sourceType === "K" &&
          attribute(question.source, "slider_layout") === "1"
        ) {
          const min = Number(attribute(question.source, "slider_min") || "0");
          const max = Number(attribute(question.source, "slider_max") || "100");
          const step = Number(attribute(question.source, "slider_accuracy") || "1");
          const position = (number - min) / step;
          if (number < min || number > max || Math.abs(position - Math.round(position)) > 1e-7)
            errors.push("La valeur ne respecte pas les bornes ou le pas du curseur.");
        }
        if (
          question.source.sourceType === "K" &&
          attribute(question.source, "slider_layout") !== "1"
        ) {
          for (const [name, constraint] of [
            ["slider_min", "min_num_value_n"],
            ["slider_max", "max_num_value_n"],
          ] as const) {
            const setting = attribute(question.source, name);
            if (setting === "" || question.constraints.has(constraint)) continue;
            const limit = Number(setting);
            if (
              (name === "slider_min" && number < limit) ||
              (name === "slider_max" && number > limit)
            )
              errors.push("La valeur ne respecte pas les bornes numériques.");
          }
        }
        if (question.source.sourceType === ":" && !isCheckboxMatrix(question.source)) {
          if (attribute(question.source, "input_boxes") === "1") {
            const { min, max } = matrixInputBounds(question.source);
            if ((min !== undefined && number < min) || (max !== undefined && number > max))
              errors.push("La valeur ne respecte pas les bornes de la matrice.");
            continue;
          }
          const { min, max, step } = matrixRange(question.source);
          const position = (number - min) / step;
          if (number < min || number > max || Math.abs(position - Math.round(position)) > 1e-7)
            errors.push("La valeur ne respecte pas les bornes ou le pas de la matrice.");
        }
      } catch {
        errors.push("Un nombre valide est attendu.");
      }
    }
    if (cell.kind === "date" && !isDate(value)) errors.push("Une date valide est attendue.");
  }
  return errors;
}

function validateConstraints(
  question: RuntimeQuestion,
  cells: ResponseCell[],
  answers: Answers,
): string[] {
  const errors: string[] = [];
  const primary = cells.filter((cell) => cell.role !== "comment" && cell.role !== "computed");
  const values = primary.map((cell) => answers.get(cell.key) ?? "").filter((value) => value !== "");
  let answerCount = values.length;
  if (isCheckboxMatrix(question.source))
    answerCount = values.filter((value) => value === "1").length;
  if (question.source.sourceType === "1") {
    const rows = new Set(primary.map((cell) => cell.rowId));
    answerCount = [...rows].filter((row) =>
      primary
        .filter((cell) => cell.rowId === row)
        .every((cell) => (answers.get(cell.key) ?? "") !== ""),
    ).length;
  }
  const numericValues = cells
    .filter((cell) => cell.kind === "number")
    .map((cell) => answers.get(cell.key) ?? "")
    .filter((value) => value !== "");
  for (const [name, expression] of question.constraints) {
    try {
      const result = expression.evaluate(answerLookup(answers));
      if (name === "em_validation_q") {
        if (!truth(result))
          errors.push(
            attribute(question.source, "em_validation_q_tip") ||
              "La règle de validation de cette question n’est pas satisfaite.",
          );
        continue;
      }
      const limit = numeric(result);
      if (name === "min_answers" && answerCount < limit)
        errors.push(`Sélectionnez au moins ${limit} réponse(s).`);
      if (name === "max_answers" && answerCount > limit)
        errors.push(`Sélectionnez au plus ${limit} réponse(s).`);
      for (const cell of cells) {
        const value = answers.get(cell.key) ?? "";
        if (value === "") continue;
        if (name === "maximum_chars" && [...value].length > limit)
          errors.push(`La réponse est limitée à ${limit} caractères.`);
        if (cell.kind !== "number") continue;
        if (name === "min_num_value_n" && numeric(value) < limit)
          errors.push(`La valeur minimale est ${limit}.`);
        if (name === "max_num_value_n" && numeric(value) > limit)
          errors.push(`La valeur maximale est ${limit}.`);
      }
      if (
        ["min_num_value", "max_num_value", "equals_num_value"].includes(name) &&
        numericValues.length > 0
      ) {
        const sum = numericValues.reduce((total, value) => total + numeric(value), 0);
        if (
          (name === "min_num_value" && sum < limit) ||
          (name === "max_num_value" && sum > limit) ||
          (name === "equals_num_value" && Math.abs(sum - limit) > 1e-9)
        )
          errors.push(`La somme des réponses ne respecte pas la contrainte (${limit}).`);
      }
    } catch (error) {
      errors.push(errorMessage(error));
    }
  }
  return errors;
}

function isUnanswered(question: RuntimeQuestion, cells: ResponseCell[], answers: Answers): boolean {
  const primary = cells.filter((cell) => ["answer", "rank"].includes(cell.role));
  if (["M", "P"].includes(question.source.sourceType)) {
    return !cells.some((cell) => cell.role !== "comment" && (answers.get(cell.key) ?? "") !== "");
  }
  if (isCheckboxMatrix(question.source)) {
    const rows = new Set(primary.map((cell) => cell.rowId));
    return [...rows].some(
      (row) => !primary.some((cell) => cell.rowId === row && answers.get(cell.key) === "1"),
    );
  }
  if (
    question.source.sourceType === "R" &&
    ["min_answers", "max_answers"].some(
      (name) => attribute(question.source, name) !== "" && attribute(question.source, name) !== "0",
    )
  ) {
    return primary.length > 0 && !primary.some((cell) => (answers.get(cell.key) ?? "") !== "");
  }
  return primary.some((cell) => (answers.get(cell.key) ?? "") === "");
}

export function validate(
  runtime: SurveyRuntime,
  snapshot: Snapshot,
  groupId?: string,
  questionIds?: ReadonlySet<string>,
): Validation {
  const errors = new Map<string, string[]>();
  const warnings = new Map<string, string[]>();
  for (const [id, question] of runtime.questions) {
    if (
      !snapshot.relevant.has(id) ||
      ["X", "*"].includes(question.source.sourceType) ||
      question.issues.length > 0
    )
      continue;
    if (groupId !== undefined && question.source.groupId !== groupId) continue;
    if (questionIds !== undefined && !questionIds.has(id)) continue;
    const cells = question.cells.filter((cell) => snapshot.activeFields.has(cell.key));
    const questionErrors = validateValues(
      question,
      cells,
      snapshot.answers,
      runtime.study.languages,
    );
    const unanswered = isUnanswered(question, cells, snapshot.answers);
    if (unanswered && question.source.mandatory === "strict")
      questionErrors.push("Cette question est obligatoire.");
    if (unanswered && question.source.mandatory === "soft")
      warnings.set(id, ["Il est recommandé de répondre à cette question."]);
    if (
      question.source.other &&
      ["L", "!"].includes(question.source.sourceType) &&
      snapshot.answers.get(question.source.code) === "-oth-" &&
      attribute(question.source, "other_comment_mandatory") !== "0" &&
      (snapshot.answers.get(`${question.source.code}_other`) ?? "") === ""
    )
      questionErrors.push("Précisez la réponse « autre ».");
    if (question.source.sourceType === "R") {
      const values = cells
        .map((cell) => snapshot.answers.get(cell.key) ?? "")
        .filter((value) => value !== "");
      if (new Set(values).size !== values.length)
        questionErrors.push("Chaque choix doit occuper une seule position du classement.");
      let blank = false;
      for (const cell of cells) {
        const value = snapshot.answers.get(cell.key) ?? "";
        if (value === "") blank = true;
        else if (blank) {
          questionErrors.push(
            "Le classement ne doit pas contenir de position vide entre deux choix.",
          );
          break;
        }
      }
    }
    questionErrors.push(...validateConstraints(question, cells, snapshot.answers));
    if (questionErrors.length > 0) errors.set(id, [...new Set(questionErrors)]);
  }
  return { errors, warnings };
}
