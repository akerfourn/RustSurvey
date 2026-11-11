import { compileExpression, type ExpressionNode } from "../survey/expressions.ts";
import { createRuntime } from "../survey/engine.ts";
import { toViewer, type Definition } from "./model.ts";

export interface Variable {
  name: string;
  label: string;
  kind: string;
  choices: { code: string; label: string }[];
}
export interface ConditionRow {
  variable: string;
  operator: string;
  value: string;
  literalType: "number" | "string";
}
export interface VisualCondition {
  join: "all" | "any";
  rows: ConditionRow[];
}
export const OPERATORS: ReadonlyMap<string, string> = new Map([
  ["==", "est égal à"],
  ["!=", "est différent de"],
  [">", "est supérieur à"],
  [">=", "est supérieur ou égal à"],
  ["<", "est inférieur à"],
  ["<=", "est inférieur ou égal à"],
  ["empty", "est vide"],
  ["answered", "est renseigné"],
]);

export function variableCatalog(definition: Definition): Variable[] {
  const runtime = createRuntime(toViewer(definition));
  const result: Variable[] = [];
  for (const question of runtime.questions.values())
    for (const cell of question.cells) {
      let kind: string = cell.kind;
      if (cell.role === "computed") kind = "calcul";
      result.push({
        name: cell.key,
        label: question.source.code,
        kind,
        choices: question.choices
          .filter((choice) => choice.scale === cell.scale)
          .map((choice) => ({
            code: choice.code,
            label:
              choice.labels.find((item) => item.language === definition.language)?.text ??
              choice.code,
          })),
      });
    }
  return result;
}
function comparison(node: ExpressionNode): ConditionRow | null {
  if (
    node.kind === "unary" &&
    (node.operator === "!" || node.operator === "not") &&
    node.child.kind === "call" &&
    node.child.name === "is_empty"
  ) {
    const value = node.child.arguments[0];
    if (value?.kind === "variable")
      return { variable: value.name, operator: "answered", value: "", literalType: "string" };
  }
  if (node.kind === "call" && node.name === "is_empty") {
    const value = node.arguments[0];
    if (value?.kind === "variable")
      return { variable: value.name, operator: "empty", value: "", literalType: "string" };
  }
  const aliases: Record<string, string> = {
    eq: "==",
    ne: "!=",
    lt: "<",
    le: "<=",
    gt: ">",
    ge: ">=",
  };
  if (node.kind !== "binary" || node.left.kind !== "variable" || node.right.kind !== "literal")
    return null;
  const operator = aliases[node.operator] ?? node.operator;
  if (
    !OPERATORS.has(operator) ||
    node.right.value === null ||
    typeof node.right.value === "boolean"
  )
    return null;
  return {
    variable: node.left.name,
    operator,
    value: String(node.right.value),
    literalType: typeof node.right.value as "number" | "string",
  };
}
export function visualCondition(source: string): VisualCondition | null {
  if (source.trim() === "" || source.trim() === "1") return { join: "all", rows: [] };
  let tree: ExpressionNode;
  try {
    tree = compileExpression(source).tree;
  } catch {
    return null;
  }
  let join: VisualCondition["join"] = "all";
  if (tree.kind === "binary" && ["or", "||"].includes(tree.operator)) join = "any";
  const rows: ConditionRow[] = [];
  function visit(node: ExpressionNode): boolean {
    if (
      node.kind === "binary" &&
      ((join === "all" && ["and", "&&"].includes(node.operator)) ||
        (join === "any" && ["or", "||"].includes(node.operator)))
    )
      return visit(node.left) && visit(node.right);
    const row = comparison(node);
    if (row === null) return false;
    rows.push(row);
    return true;
  }
  if (!visit(tree) || rows.length > 20) return null;
  return { join, rows };
}
function quote(value: string): string {
  return (
    "'" +
    value
      .replaceAll("\\", "\\\\")
      .replaceAll("'", "\\'")
      .replaceAll("\n", "\\n")
      .replaceAll("\r", "\\r")
      .replaceAll("\t", "\\t") +
    "'"
  );
}
export function conditionExpression(condition: VisualCondition): string {
  if (condition.rows.length === 0) return "1";
  const rows = condition.rows.map((row) => {
    if (row.operator === "empty") return `is_empty(${row.variable})`;
    if (row.operator === "answered") return `!is_empty(${row.variable})`;
    let literal = quote(row.value);
    if (
      row.literalType === "number" &&
      row.value.trim() !== "" &&
      Number.isFinite(Number(row.value))
    )
      literal = String(Number(row.value));
    return `(${row.variable} ${row.operator} ${literal})`;
  });
  let separator = " && ";
  if (condition.join === "any") separator = " || ";
  return rows.join(separator);
}
export function expressionIssues(value: string, variables: readonly Variable[]): string[] {
  try {
    const expression = compileExpression(value.trim() || "1");
    return [...expression.variables]
      .filter((name) => !variables.some((item) => item.name === name.replace(/\.NAOK$/, "")))
      .map((name) => `Variable inconnue : ${name}.`);
  } catch (error) {
    return [error instanceof Error ? error.message : "Expression invalide."];
  }
}
