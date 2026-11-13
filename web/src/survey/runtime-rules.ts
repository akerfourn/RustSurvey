import { compileExpression, type Expression } from "./expressions.ts";
import {
  attribute,
  CONSTRAINT_ATTRIBUTES,
  hasActiveTimer,
  INACTIVE_ATTRIBUTES,
  INTERACTION_ATTRIBUTES,
  isPassiveAttribute,
  TEXT_ATTRIBUTES,
} from "./attributes.ts";
import type { RuntimeQuestion } from "./engine.ts";
import type { SurveyGroup, SurveyStudy } from "./types.ts";

export function variableName(name: string): string {
  return name.replace(/\.NAOK$/, "");
}

export function compileRule(
  source: string,
  fields: ReadonlyMap<string, string>,
  issues: string[],
): Expression | null {
  try {
    const expression = compileExpression(source);
    for (const name of expression.variables) {
      if (!fields.has(variableName(name)))
        throw new Error(`Variable non prise en charge : ${name}.`);
    }
    return expression;
  } catch (error) {
    issues.push(error instanceof Error ? error.message : "Expression invalide.");
    return null;
  }
}

export function prepareRules(
  question: RuntimeQuestion,
  group: SurveyGroup | undefined,
  fields: Map<string, string>,
  study: SurveyStudy,
): void {
  if (
    question.source.randomizationGroup !== "" ||
    (group !== undefined && group.randomizationGroup !== "")
  ) {
    question.issues.push("Ordre aléatoire de groupes ou questions non pris en charge.");
  }
  for (const source of [group?.relevance ?? "1", question.source.relevance]) {
    const expression = compileRule(source.trim() || "1", fields, question.issues);
    if (expression !== null) question.relevance.push(expression);
  }
  for (const row of question.subquestions) {
    const expression = compileRule(row.relevance.trim() || "1", fields, question.issues);
    if (expression !== null) question.subquestionRules.set(row.id, expression);
  }
  if (hasActiveTimer(question.source))
    question.issues.push("Minuterie active non prise en charge.");
  for (const item of question.source.attributes) {
    if (item.value === "") continue;
    if (INACTIVE_ATTRIBUTES.has(item.name) && item.value === "0") continue;
    if (item.name === "maximum_chars" && item.value.trim() === "0") continue;
    if (CONSTRAINT_ATTRIBUTES.has(item.name)) {
      if (item.language !== null && item.language !== "") {
        question.issues.push(`Contrainte localisée non prise en charge : ${item.name}.`);
        continue;
      }
      const expression = compileRule(item.value, fields, question.issues);
      if (expression !== null) question.constraints.set(item.name, expression);
    } else if (INTERACTION_ATTRIBUTES.has(item.name)) {
      if (!["0", "1"].includes(item.value) || (item.language !== null && item.language !== "")) {
        question.issues.push(`Valeur d’attribut non prise en charge : ${item.name}.`);
      }
    } else if (
      ["slider_min", "slider_max", "slider_accuracy"].includes(item.name) &&
      question.source.sourceType === "K"
    ) {
      if (!Number.isFinite(Number(item.value)) || item.value.trim() === "")
        question.issues.push("Borne de curseur non littérale ou invalide.");
    } else if (
      item.name === "commented_checkbox_auto" &&
      item.value === "1" &&
      question.source.sourceType === "P"
    ) {
      continue;
    } else if (item.name === "equation" && question.source.sourceType === "*") {
      continue;
    } else if (
      item.name === "multiflexible_checkbox" &&
      question.source.sourceType === ":" &&
      ["0", "1"].includes(item.value)
    ) {
      continue;
    } else if (
      ["multiflexible_min", "multiflexible_max", "multiflexible_step"].includes(item.name) &&
      question.source.sourceType === ":"
    ) {
      continue;
    } else if (
      item.name === "commented_checkbox" &&
      ["checked", "unchecked", "allways"].includes(item.value)
    ) {
      continue;
    } else if (["array_filter", "array_filter_exclude"].includes(item.name)) {
      if (
        !["M", "P", "Q", "K", "A", "B", "C", "E", "F", "H", "1", ":", ";"].includes(
          question.source.sourceType,
        )
      ) {
        question.issues.push("Filtre de choix ou de classement non pris en charge.");
        continue;
      }
      for (const code of item.value
        .split(";")
        .map((part) => part.trim())
        .filter((part) => part !== "")) {
        const source = study.questions.find((candidate) => candidate.code === code);
        if (source === undefined || !["M", "P"].includes(source.sourceType)) {
          question.issues.push(`Le filtre ${code} doit désigner un choix multiple.`);
        } else
          question.filters.push({
            sourceId: source.id,
            code,
            exclude: item.name === "array_filter_exclude",
            selections: new Set(
              study.subquestions
                .filter((row) => row.parentId === source.id && row.scale === 0)
                .map((row) => `${code}_${row.code}`),
            ),
          });
      }
    } else if (TEXT_ATTRIBUTES.has(item.name) || isPassiveAttribute(item.name)) {
      continue;
    } else {
      question.issues.push(`Attribut non pris en charge : ${item.name}.`);
    }
  }
  if (question.source.sourceType === "K" && attribute(question.source, "slider_layout") === "1") {
    const min = Number(attribute(question.source, "slider_min") || "0");
    const max = Number(attribute(question.source, "slider_max") || "100");
    const step = Number(attribute(question.source, "slider_accuracy") || "1");
    if (
      ![min, max, step].every(Number.isFinite) ||
      max < min ||
      step <= 0 ||
      !Number.isFinite((max - min) / step)
    )
      question.issues.push("Intervalle de curseur invalide.");
  }
  if (question.source.sourceType === "*") {
    const source =
      attribute(question.source, "equation") ||
      question.source.labels.find((label) => label.language === study.baseLanguage)?.text ||
      "";
    const expression = source.trim().replace(/^\{([\s\S]*)\}$/, "$1");
    question.equation = compileRule(expression, fields, question.issues);
    if (
      attribute(question.source, "equation") === "" &&
      question.source.labels.some((label) => label.text !== source)
    ) {
      question.issues.push("Équation différente selon la langue non prise en charge.");
    }
  }
}
