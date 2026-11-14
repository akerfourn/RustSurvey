import { compileExpression, type Expression, type Lookup } from "./expressions.ts";
import { stringValue } from "./scalars.ts";

export interface DefaultTemplate {
  parts: (string | Expression)[];
  variables: Set<string>;
}

export interface DefaultBudget {
  remaining: number;
}

export function compileDefaultTemplate(source: string): DefaultTemplate {
  if (source.length > 4096) throw new Error("Valeur par défaut limitée à 4096 unités UTF-16.");
  const parts: DefaultTemplate["parts"] = [];
  const variables = new Set<string>();
  let remaining = source;
  let expressions = 0;
  while (remaining.includes("{")) {
    const open = remaining.indexOf("{");
    if (open > 0) parts.push(remaining.slice(0, open));
    remaining = remaining.slice(open + 1);
    const close = expressionEnd(remaining);
    const expression = compileExpression(remaining.slice(0, close));
    for (const name of expression.variables) variables.add(name);
    parts.push(expression);
    expressions += 1;
    if (expressions > 100) throw new Error("Maximum de 100 expressions par défaut.");
    remaining = remaining.slice(close + 1);
  }
  parts.push(remaining);
  return { parts, variables };
}

export function renderDefaultTemplate(
  template: DefaultTemplate,
  lookup: Lookup,
  budget: DefaultBudget,
): string {
  let output = "";
  for (const part of template.parts) {
    let text: string;
    if (typeof part === "string") text = part;
    else text = stringValue(part.evaluate(lookup));
    budget.remaining -= Math.max(1, text.length);
    if (budget.remaining < 0) throw new Error("Budget d’initialisation dépassé.");
    if (output.length + text.length > 65_536)
      throw new Error("Résultat de valeur par défaut trop long.");
    output += text;
  }
  return output;
}

function expressionEnd(source: string): number {
  let quote: string | null = null;
  let escaped = false;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (quote !== null) {
      if (character === "\\") escaped = true;
      else if (character === quote) quote = null;
      continue;
    }
    if (character === "'" || character === '"') quote = character;
    else if (character === "}") return index;
    else if (character === "{")
      throw new Error("Accolades imbriquées non prises en charge dans un défaut.");
  }
  throw new Error("Expression de valeur par défaut non fermée.");
}
