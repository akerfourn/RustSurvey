import { attribute, isCheckboxMatrix } from "./attributes.ts";
import { isChoiceArray, isMatrix } from "./question-kinds.ts";
import type { SurveyChoice, SurveyQuestion, SurveySubquestion } from "./types.ts";

export interface ResponseCell {
  key: string;
  kind: "text" | "number" | "date" | "choice" | "multiple" | "checkbox";
  role: "answer" | "comment" | "other" | "rank" | "computed";
  rowId: string | null;
  columnId: string | null;
  scale: number;
}

export const MAX_QUESTION_CELLS = 2000;
export const MAX_SURVEY_CELLS = 20000;

export function fieldName(
  question: SurveyQuestion,
  row: SurveySubquestion,
  column?: SurveySubquestion,
): string {
  let key = `${question.code}_${row.code}`;
  if (column !== undefined) key += `_${column.code}`;
  return key;
}

function scalarKind(question: SurveyQuestion): ResponseCell["kind"] {
  const type = question.sourceType;
  if (
    ["N", "K", ":"].includes(type) ||
    (["S", "T", "U", "Q", ";", "*"].includes(type) && attribute(question, "numbers_only") === "1")
  )
    return "number";
  if (type === "D") return "date";
  if (["L", "!", "5", "Y", "G", "O", "I"].includes(type) || isChoiceArray(type)) return "choice";
  if (type === "M" || type === "P") return "multiple";
  return "text";
}

export function createCells(
  question: SurveyQuestion,
  subquestions: SurveySubquestion[],
  issues: string[],
  choices: SurveyChoice[] = [],
): ResponseCell[] {
  const type = question.sourceType;
  if (type === "X") return [];
  const rows = subquestions.filter((item) => item.scale === 0);
  const columns = subquestions.filter((item) => item.scale === 1);
  const matrix = isMatrix(type);
  const multiple = matrix || isChoiceArray(type) || ["M", "P", "Q", "K"].includes(type);
  if (matrix && (rows.length === 0 || columns.length === 0)) {
    issues.push("La matrice nécessite un axe de lignes et un axe de colonnes.");
    return [];
  }
  let count = 1;
  if (multiple) count = rows.length;
  if (matrix) count *= columns.length;
  if (type === "1" || type === "P") count *= 2;
  if (type === "R") {
    count = choices.filter((choice) => choice.scale === 0).length;
    const maximum = Number(attribute(question, "max_subquestions"));
    if (Number.isInteger(maximum) && maximum > 0) count = Math.min(count, maximum);
  }
  if (type === "O") count += 1;
  if (question.other) count += 1;
  if (question.other && type === "P") count += 1;
  if (count > MAX_QUESTION_CELLS) {
    issues.push(`Question limitée à ${MAX_QUESTION_CELLS} cellules de réponse.`);
    return [];
  }
  if (multiple && rows.length === 0) issues.push("Aucune sous-question exploitable.");
  const cells: ResponseCell[] = [];
  function add(
    key: string,
    kind: ResponseCell["kind"],
    role: ResponseCell["role"] = "answer",
    rowId: string | null = null,
    columnId: string | null = null,
    scale = 0,
  ): void {
    cells.push({ key, kind, role, rowId, columnId, scale });
  }
  if (type === "R") {
    for (let index = 0; index < count; index += 1)
      add(`${question.code}_${index + 1}`, "choice", "rank");
  } else if (!multiple) {
    let role: ResponseCell["role"] = "answer";
    if (type === "*") role = "computed";
    add(question.code, scalarKind(question), role);
    if (type === "O") add(`${question.code}_comment`, "text", "comment");
  } else {
    for (const row of rows) {
      const key = fieldName(question, row);
      if (matrix) {
        for (const column of columns) {
          let kind = scalarKind(question);
          if (isCheckboxMatrix(question)) kind = "checkbox";
          add(fieldName(question, row, column), kind, "answer", row.id, column.id);
        }
      } else if (type === "1") {
        add(`${key}_0`, "choice", "answer", row.id, null, 0);
        add(`${key}_1`, "choice", "answer", row.id, null, 1);
      } else {
        add(key, scalarKind(question), "answer", row.id);
        if (type === "P") add(`${key}comment`, "text", "comment", row.id);
      }
    }
  }
  if (question.other) {
    add(`${question.code}_other`, "text", "other");
    if (type === "P") add(`${question.code}_othercomment`, "text", "comment");
  }
  return cells;
}
