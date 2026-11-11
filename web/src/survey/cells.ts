import { isCheckboxMatrix } from "./attributes.ts";
import type { SurveyQuestion, SurveySubquestion } from "./types.ts";

export interface ResponseCell {
  key: string;
  kind: "text" | "number" | "date" | "choice" | "multiple" | "checkbox";
  rowId: string | null;
  columnId: string | null;
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

function scalarKind(type: string): ResponseCell["kind"] {
  if (["N", "K"].includes(type)) return "number";
  if (type === "D") return "date";
  if (["L", "!", "5", "Y", "G", "F", "H"].includes(type)) return "choice";
  if (type === "M") return "multiple";
  return "text";
}

export function createCells(
  question: SurveyQuestion,
  subquestions: SurveySubquestion[],
  issues: string[],
): ResponseCell[] {
  const type = question.sourceType;
  if (type === "X") return [];
  const rows = subquestions.filter((item) => item.scale === 0);
  const columns = subquestions.filter((item) => item.scale === 1);
  const matrix = isCheckboxMatrix(question);
  if (type === ":" && !matrix) {
    issues.push("Seules les matrices à cases à cocher sont prises en charge pour le type :.");
    return [];
  }
  if (matrix && (rows.length === 0 || columns.length === 0)) {
    issues.push("La matrice nécessite un axe de lignes et un axe de colonnes.");
    return [];
  }
  const multiple = matrix || ["M", "Q", "K", "F", "H"].includes(type);
  let count = 1;
  if (multiple) count = rows.length;
  if (matrix) count *= columns.length;
  if (count > MAX_QUESTION_CELLS) {
    issues.push(`Question limitée à ${MAX_QUESTION_CELLS} cellules de réponse.`);
    return [];
  }
  if (multiple && rows.length === 0) issues.push("Aucune sous-question exploitable.");
  if (!multiple)
    return [{ key: question.code, kind: scalarKind(type), rowId: null, columnId: null }];
  const cells: ResponseCell[] = [];
  for (const row of rows) {
    if (!matrix) {
      cells.push({
        key: fieldName(question, row),
        kind: scalarKind(type),
        rowId: row.id,
        columnId: null,
      });
      continue;
    }
    for (const column of columns) {
      cells.push({
        key: fieldName(question, row, column),
        kind: "checkbox",
        rowId: row.id,
        columnId: column.id,
      });
    }
  }
  return cells;
}
