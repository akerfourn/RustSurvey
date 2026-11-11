import { DEFERRED_PRESENTATION_ATTRIBUTES, TEXT_ATTRIBUTES } from "./attributes.ts";
import { compileExpression, type Expression } from "./expressions.ts";
import { answerLookup, errorMessage, type Snapshot } from "./engine.ts";
import type { SurveyRuntime } from "./engine.ts";
import { localized, plainText, textIssues } from "./text.ts";

export function studyTextIssues(runtime: SurveyRuntime): string[] {
  const issues: string[] = [];
  const texts: (string | null)[] = [];
  for (const group of runtime.study.groups)
    for (const label of group.labels) texts.push(label.name, label.description);
  for (const question of [...runtime.study.questions, ...runtime.study.subquestions])
    for (const label of question.labels) texts.push(label.text, label.help);
  for (const choice of runtime.study.answers)
    for (const label of choice.labels) texts.push(label.text);
  for (const message of runtime.study.messages) texts.push(message.welcome, message.end);
  for (const question of runtime.study.questions)
    for (const item of question.attributes)
      if (TEXT_ATTRIBUTES.has(item.name)) texts.push(item.value);
  for (const text of texts) {
    issues.push(...textIssues(plainText(text), runtime.fields));
    if (issues.length >= 200) {
      issues.push("Rapport de textes limité à 200 diagnostics.");
      break;
    }
  }
  return [...new Set(issues)];
}

export function presentationNotices(runtime: SurveyRuntime): string[] {
  const notices = new Set<string>();
  const cosmetic = new Set([
    "cssclass",
    "page_break",
    "answer_width",
    "answer_width_bycolumn",
    "hide_title",
  ]);
  for (const question of runtime.questions.values())
    for (const item of question.source.attributes) {
      if (
        item.value === "" ||
        item.value === "0" ||
        !DEFERRED_PRESENTATION_ATTRIBUTES.has(item.name) ||
        !cosmetic.has(item.name)
      )
        continue;
      notices.add(`${question.source.code} : présentation non reproduite (${item.name}).`);
      if (notices.size >= 200)
        return [...notices, "Rapport de présentation limité à 200 diagnostics."];
    }
  return [...notices];
}

export interface CompiledText {
  ownerId: string | null;
  rowId: string | null;
  groupId: string | null;
  expressions: Expression[];
}

export function compileRuntimeTexts(runtime: SurveyRuntime, language: string): CompiledText[] {
  const result: CompiledText[] = [];
  function add(
    text: string | null | undefined,
    ownerId: string | null,
    rowId: string | null = null,
    groupId: string | null = null,
  ): void {
    const expressions: Expression[] = [];
    for (const match of plainText(text).matchAll(/\{([^{}]+)\}/g)) {
      if (expressions.length >= 100) break;
      try {
        expressions.push(compileExpression(match[1] ?? ""));
      } catch {
        continue;
      }
    }
    if (expressions.length > 0) result.push({ ownerId, rowId, groupId, expressions });
  }
  for (const group of runtime.groups) {
    const label = localized(group.labels, language, runtime.study.baseLanguage);
    add(label?.name, null, null, group.id);
    add(label?.description, null, null, group.id);
  }
  for (const question of runtime.questions.values()) {
    const label = localized(question.source.labels, language, runtime.study.baseLanguage);
    add(label?.text, question.source.id);
    add(label?.help, question.source.id);
    for (const row of question.subquestions)
      add(
        localized(row.labels, language, runtime.study.baseLanguage)?.text,
        question.source.id,
        row.id,
      );
    for (const choice of question.choices)
      add(localized(choice.labels, language, runtime.study.baseLanguage)?.text, question.source.id);
    for (const item of question.source.attributes)
      if (TEXT_ATTRIBUTES.has(item.name)) add(item.value, question.source.id);
  }
  return result;
}

export function textExecutionIssues(texts: readonly CompiledText[], snapshot: Snapshot): string[] {
  const issues = new Set<string>();
  for (const text of texts) {
    if (text.ownerId !== null && !snapshot.visible.has(text.ownerId)) continue;
    if (text.rowId !== null && !snapshot.visibleSubquestions.has(text.rowId)) continue;
    if (text.groupId !== null && !snapshot.groups.some((group) => group.id === text.groupId))
      continue;
    for (const expression of text.expressions) {
      try {
        expression.evaluate(answerLookup(snapshot.answers));
      } catch (error) {
        issues.add(`Texte importé : ${errorMessage(error)}`);
      }
    }
  }
  return [...issues];
}
