import { compileExpression, truth, type Expression, type Lookup } from "./expressions.ts";
import type {
  Answers,
  SurveyChoice,
  SurveyGroup,
  SurveyQuestion,
  SurveyStudy,
  SurveySubquestion,
} from "./types.ts";

export interface RuntimeQuestion {
  source: SurveyQuestion;
  subquestions: SurveySubquestion[];
  choices: SurveyChoice[];
  fields: string[];
  relevance: Expression[];
  constraints: Map<string, Expression>;
  issues: string[];
}

export interface SurveyRuntime {
  study: SurveyStudy;
  groups: SurveyGroup[];
  questions: Map<string, RuntimeQuestion>;
  fields: Map<string, string>;
  order: string[];
  issues: string[];
}

export interface Snapshot {
  answers: Map<string, string>;
  visible: Set<string>;
  groups: SurveyGroup[];
  issues: string[];
}

const SUPPORTED_TYPES = new Set([
  "S",
  "T",
  "U",
  "N",
  "D",
  "L",
  "!",
  "5",
  "Y",
  "G",
  "M",
  "Q",
  "K",
  "X",
]);
const MULTI_TYPES = new Set(["M", "Q", "K"]);
const CONSTRAINTS = new Set([
  "min_num_value_n",
  "max_num_value_n",
  "min_answers",
  "max_answers",
  "max_characters",
  "em_validation_q",
]);
const PRESENTATION_ATTRIBUTES = new Set([
  "display_rows",
  "text_input_width",
  "answer_width",
  "question_template",
  "cssclass",
  "page_break",
  "hide_tip",
  "hide_title",
  "hide_help",
  "display_columns",
  "scale_export",
  "statistics_showgraph",
  "statistics_graphtype",
  "public_statistics",
  "chart_type",
]);

export function fieldName(question: SurveyQuestion, subquestion: SurveySubquestion): string {
  return `${question.code}_${subquestion.code}`;
}

export function variableName(name: string): string {
  return name.replace(/\.NAOK$/, "");
}

export function attribute(question: SurveyQuestion, name: string): string {
  return (
    question.attributes.find(
      (item) => item.name === name && (item.language === null || item.language === ""),
    )?.value ?? ""
  );
}

function rule(source: string, fields: Map<string, string>, issues: string[]): Expression | null {
  try {
    const expression = compileExpression(source);
    for (const name of expression.variables) {
      if (!fields.has(variableName(name)))
        throw new Error(`Variable non prise en charge : ${name}.`);
    }
    return expression;
  } catch (error) {
    issues.push(errorMessage(error));
    return null;
  }
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return "Une règle n’a pas pu être évaluée.";
}

function prepareQuestion(study: SurveyStudy, question: SurveyQuestion): RuntimeQuestion {
  const subquestions = study.subquestions
    .filter((item) => item.parentId === question.id)
    .sort((a, b) => a.order - b.order);
  const choices = study.answers
    .filter((item) => item.questionId === question.id && item.scale === 0)
    .sort((a, b) => a.order - b.order);
  let fields = [question.code];
  if (MULTI_TYPES.has(question.sourceType))
    fields = subquestions.map((item) => fieldName(question, item));
  if (question.sourceType === "X") fields = [];
  const issues: string[] = [];
  if (!SUPPORTED_TYPES.has(question.sourceType))
    issues.push(`Type ${question.sourceType} : saisie non prise en charge.`);
  if (question.other) issues.push("Le champ « autre » natif n’est pas encore pris en charge.");
  if (question.validation !== "")
    issues.push("Validation par expression régulière non prise en charge.");
  if (MULTI_TYPES.has(question.sourceType) && subquestions.length === 0)
    issues.push("Aucune sous-question exploitable.");
  if (["L", "!"].includes(question.sourceType) && choices.length === 0)
    issues.push("Aucun choix de réponse exploitable.");
  if (subquestions.some((item) => item.scale !== 0 || !["", "1"].includes(item.relevance.trim()))) {
    issues.push("Échelle ou pertinence de sous-question non prise en charge.");
  }
  return {
    source: question,
    subquestions,
    choices,
    fields,
    relevance: [],
    constraints: new Map(),
    issues,
  };
}

function prepareRules(
  question: RuntimeQuestion,
  group: SurveyGroup | undefined,
  fields: Map<string, string>,
): void {
  if (
    question.source.randomizationGroup !== "" ||
    (group !== undefined && group.randomizationGroup !== "")
  ) {
    question.issues.push("Ordre aléatoire de groupes ou questions non pris en charge.");
  }
  for (const source of [group?.relevance ?? "1", question.source.relevance]) {
    const expression = rule(source.trim() || "1", fields, question.issues);
    if (expression !== null) question.relevance.push(expression);
  }
  for (const item of question.source.attributes) {
    if (item.value === "") continue;
    if (CONSTRAINTS.has(item.name)) {
      if (item.language !== null && item.language !== "") {
        question.issues.push(`Contrainte localisée non prise en charge : ${item.name}.`);
        continue;
      }
      const expression = rule(item.value, fields, question.issues);
      if (expression !== null) question.constraints.set(item.name, expression);
    } else if (
      item.value !== "0" &&
      item.name !== "num_value_int_only" &&
      !PRESENTATION_ATTRIBUTES.has(item.name)
    ) {
      question.issues.push(`Attribut non pris en charge : ${item.name}.`);
    }
  }
}

function sortDependencies(
  questions: Map<string, RuntimeQuestion>,
  fields: Map<string, string>,
): string[] {
  const dependents = new Map<string, Set<string>>();
  const degrees = new Map<string, number>();
  for (const [id, question] of questions) {
    const dependencies = new Set<string>();
    for (const expression of question.relevance) {
      for (const variable of expression.variables) {
        const owner = fields.get(variableName(variable));
        if (owner !== undefined) dependencies.add(owner);
      }
    }
    degrees.set(id, dependencies.size);
    for (const dependency of dependencies) {
      const next = dependents.get(dependency) ?? new Set<string>();
      next.add(id);
      dependents.set(dependency, next);
    }
  }
  const order: string[] = [];
  for (const [id, degree] of degrees) {
    if (degree === 0) order.push(id);
  }
  for (let position = 0; position < order.length; position += 1) {
    for (const id of dependents.get(order[position] ?? "") ?? []) {
      const degree = (degrees.get(id) ?? 1) - 1;
      degrees.set(id, degree);
      if (degree === 0) order.push(id);
    }
  }
  for (const [id, degree] of degrees) {
    if (degree > 0) {
      questions
        .get(id)
        ?.issues.push("Cycle de pertinence, ou dépendance à un cycle : simulation impossible.");
      order.push(id);
    }
  }
  return order;
}

export function createRuntime(study: SurveyStudy): SurveyRuntime {
  const groups = [...study.groups].sort((a, b) => a.order - b.order);
  const questions = new Map<string, RuntimeQuestion>();
  const fields = new Map<string, string>();
  const issues: string[] = [];
  if (study.questions.length > 2000) {
    issues.push("Aperçu limité à 2 000 questions ; ce questionnaire peut seulement être inspecté.");
    return { study, groups, questions, fields, order: [], issues };
  }
  for (const source of [...study.questions].sort((a, b) => a.order - b.order)) {
    const question = prepareQuestion(study, source);
    questions.set(source.id, question);
    for (const field of question.fields) {
      if (fields.has(field)) issues.push(`Cellule ambiguë : ${field}.`);
      fields.set(field, source.id);
    }
  }
  for (const question of questions.values()) {
    prepareRules(
      question,
      groups.find((group) => group.id === question.source.groupId),
      fields,
    );
  }
  const order = sortDependencies(questions, fields);
  return { study, groups, questions, fields, order, issues };
}

export function answerLookup(answers: Answers): Lookup {
  return (name) => answers.get(variableName(name)) ?? "";
}

export function evaluateSnapshot(runtime: SurveyRuntime, answers: Answers): Snapshot {
  const effective = new Map<string, string>();
  for (const [field, value] of answers) {
    if (runtime.fields.has(field)) effective.set(field, value);
  }
  const visible = new Set<string>();
  const issues = [...runtime.issues];
  for (const id of runtime.order) {
    const question = runtime.questions.get(id);
    if (question === undefined) continue;
    let relevant: boolean;
    try {
      relevant = question.relevance.every((expression) =>
        truth(expression.evaluate(answerLookup(effective))),
      );
    } catch (error) {
      issues.push(`${question.source.code} : ${errorMessage(error)}`);
      relevant = false;
    }
    if (relevant || question.issues.length > 0) visible.add(id);
    if (!relevant || question.issues.length > 0) {
      for (const field of question.fields) effective.delete(field);
    }
  }
  const groups = runtime.groups.filter((group) =>
    [...runtime.questions.values()].some(
      (question) => question.source.groupId === group.id && visible.has(question.source.id),
    ),
  );
  return { answers: effective, visible, groups, issues };
}

export function runtimeIssues(runtime: SurveyRuntime, snapshot: Snapshot): string[] {
  const issues = [...snapshot.issues];
  for (const question of runtime.questions.values()) {
    for (const issue of question.issues) issues.push(`${question.source.code} : ${issue}`);
  }
  return issues;
}
