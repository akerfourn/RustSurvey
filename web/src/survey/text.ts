import { answerLookup, errorMessage } from "./engine.ts";
import { compileExpression } from "./expressions.ts";
import type { Answers } from "./types.ts";

export function localized<T extends { language: string }>(
  labels: readonly T[],
  language: string,
  base: string | null,
): T | undefined {
  return (
    labels.find((label) => label.language === language) ??
    labels.find((label) => label.language === base) ??
    labels[0]
  );
}

export function plainText(html: string | null | undefined): string {
  if (html === null || html === undefined || html === "") return "";
  const template = document.createElement("template");
  template.innerHTML = html;
  template.content
    .querySelectorAll("script,style,iframe,object,template,noscript,link,meta")
    .forEach((element) => element.remove());
  template.content
    .querySelectorAll("br,p,div,li,h1,h2,h3")
    .forEach((element) => element.append("\n"));
  return (template.content.textContent ?? "")
    .replace(/[ \t]+/g, " ")
    .replace(/\n\s*\n/g, "\n")
    .trim();
}

export interface RenderedText {
  text: string;
  issues: string[];
}

export function pipeText(
  text: string,
  answers: Answers,
  fields: ReadonlyMap<string, string>,
): RenderedText {
  const issues: string[] = [];
  let count = 0;
  const rendered = text.replace(/\{([^{}]+)\}/g, (original: string, source: string): string => {
    count += 1;
    if (count > 100) {
      if (count === 101) issues.push("Trop de substitutions dans un texte.");
      return original;
    }
    try {
      const expression = compileExpression(source);
      for (const name of expression.variables) {
        if (!fields.has(name.replace(/\.NAOK$/, "")))
          throw new Error(`Variable de texte non prise en charge : ${name}.`);
      }
      return String(expression.evaluate(answerLookup(answers)) ?? "");
    } catch (error) {
      issues.push(errorMessage(error));
      return original;
    }
  });
  return { text: rendered, issues };
}

export function textIssues(text: string, fields: ReadonlyMap<string, string>): string[] {
  const issues: string[] = [];
  const substitutions = [...text.matchAll(/\{([^{}]+)\}/g)];
  if (substitutions.length > 100) return ["Trop de substitutions dans un texte."];
  for (const substitution of substitutions) {
    try {
      const expression = compileExpression(substitution[1] ?? "");
      for (const variable of expression.variables) {
        if (!fields.has(variable.replace(/\.NAOK$/, "")))
          throw new Error(`Variable de texte non prise en charge : ${variable}.`);
      }
    } catch (error) {
      issues.push(errorMessage(error));
    }
  }
  return issues;
}
