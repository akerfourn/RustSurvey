import { truth, type Expression, type Lookup } from "./expressions.ts";
import { attribute, isCheckboxMatrix } from "./attributes.ts";
import { isChoiceArray, isMatrix, choiceCodes, isBaseTheme } from "./question-kinds.ts";
import { matrixOptions, matrixInputBounds } from "./matrix-options.ts";
import { prepareRules, variableName } from "./runtime-rules.ts";
import {
  applyChoiceFilters,
  filterAllows,
  prepareFilters,
  type ArrayFilter,
  type SelectionCache,
} from "./filters.ts";
import {
  applyComments,
  applyExclusive,
  parseExclusive,
  preserveExistingExclusiveSelection,
  type ExclusiveOptions,
} from "./exclusive.ts";
import { createCells, MAX_QUESTION_CELLS, MAX_SURVEY_CELLS, type ResponseCell } from "./cells.ts";
import {
  prepareDefaults,
  applyDefaults,
  checkInactiveDefaultReferences,
  type RuntimeDefaults,
} from "./runtime-defaults.ts";
import type { DefaultBudget } from "./default-template.ts";
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
  cells: ResponseCell[];
  relevance: Expression[];
  constraints: Map<string, Expression>;
  subquestionRules: Map<string, Expression>;
  filters: ArrayFilter[];
  equation: Expression | null;
  issues: string[];
  exclusive: ExclusiveOptions;
  defaults: RuntimeDefaults | null;
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
  initialized: Set<string>;
  answers: Map<string, string>;
  visible: Set<string>;
  relevant: Set<string>;
  activeFields: Set<string>;
  visibleSubquestions: Set<string>;
  visibleOther: Set<string>;
  activeChoices: Map<string, ReadonlySet<string>>;
  groups: SurveyGroup[];
  issues: string[];
}

const SUPPORTED_TYPES = new Set([
  "A",
  "B",
  "C",
  "E",
  "1",
  ";",
  "O",
  "P",
  "R",
  "I",
  "*",
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
  "F",
  "H",
  ":",
]);
export { attribute } from "./attributes.ts";
export { fieldName } from "./cells.ts";

export { variableName } from "./runtime-rules.ts";

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return "Une règle n’a pas pu être évaluée.";
}

function prepareQuestion(study: SurveyStudy, question: SurveyQuestion): RuntimeQuestion {
  const subquestions = study.subquestions
    .filter((item) => item.parentId === question.id)
    .sort((a, b) => a.order - b.order);
  const choices = study.answers
    .filter((item) => item.questionId === question.id)
    .sort((a, b) => a.order - b.order);
  const issues: string[] = [];
  const cells = createCells(question, subquestions, issues, choices);
  const fields = cells.map((cell) => cell.key);
  if (!isBaseTheme(question.sourceType, question.theme))
    issues.push(`Thème de question non pris en charge : ${question.theme}.`);
  if (!isBaseTheme(question.sourceType, attribute(question, "question_template")))
    issues.push("Thème de question historique non pris en charge.");
  if (!SUPPORTED_TYPES.has(question.sourceType))
    issues.push(`Type ${question.sourceType} : saisie non prise en charge.`);
  if (question.other && !["L", "!", "M", "P"].includes(question.sourceType))
    issues.push("Option autre incompatible avec ce type.");
  if (question.validation !== "")
    issues.push("Validation par expression régulière non prise en charge.");
  if (["L", "!", "O", "R", "F", "H", "1"].includes(question.sourceType) && choices.length === 0)
    issues.push("Aucun choix de réponse exploitable.");
  let displayedChoices = choiceCodes(question.sourceType, choices).length;
  if (question.mandatory === "no") displayedChoices += 1;
  if (isChoiceArray(question.sourceType) && cells.length * displayedChoices > MAX_QUESTION_CELLS) {
    issues.push(`Tableau limité à ${MAX_QUESTION_CELLS} choix affichés.`);
  }
  if (
    subquestions.some(
      (item) => item.scale !== 0 && !(isMatrix(question.sourceType) && item.scale === 1),
    )
  ) {
    issues.push("Échelle de sous-question non prise en charge.");
  }
  if (
    ["F", "H"].includes(question.sourceType) &&
    study.answers.some((item) => item.questionId === question.id && item.scale !== 0)
  ) {
    issues.push("Échelle supplémentaire de réponses non prise en charge.");
  }
  if (["L", "!", "O"].includes(question.sourceType) && choices.length > MAX_QUESTION_CELLS)
    issues.push(`Liste limitée à ${MAX_QUESTION_CELLS} choix.`);
  if (question.sourceType === "R" && cells.length * choices.length > MAX_QUESTION_CELLS)
    issues.push(`Classement limité à ${MAX_QUESTION_CELLS} choix affichés.`);
  if (question.sourceType === "1" && (cells.length / 2) * (choices.length + 2) > MAX_QUESTION_CELLS)
    issues.push(`Tableau double limité à ${MAX_QUESTION_CELLS} choix affichés.`);
  if (
    question.sourceType === "1" &&
    [0, 1].some((scale) => !choices.some((choice) => choice.scale === scale))
  ) {
    issues.push("Le tableau double nécessite deux échelles de réponses.");
  }
  if (question.sourceType === ":" && !isCheckboxMatrix(question)) {
    try {
      if (attribute(question, "input_boxes") === "1") matrixInputBounds(question);
      else matrixOptions(question, cells.length);
    } catch (error) {
      issues.push(errorMessage(error));
    }
  }
  let exclusive: ExclusiveOptions = { fields: new Set(), automatic: false };
  try {
    exclusive = parseExclusive(question, subquestions);
  } catch (error) {
    issues.push(errorMessage(error));
  }
  return {
    source: question,
    subquestions,
    choices,
    fields,
    cells,
    relevance: [],
    constraints: new Map(),
    subquestionRules: new Map(),
    filters: [],
    equation: null,
    issues,
    exclusive,
    defaults: null,
  };
}

function sortDependencies(
  questions: Map<string, RuntimeQuestion>,
  fields: Map<string, string>,
): string[] {
  const dependents = new Map<string, Set<string>>();
  const degrees = new Map<string, number>();
  for (const [id, question] of questions) {
    const dependencies = new Set<string>();
    const expressions = [...question.relevance, ...question.subquestionRules.values()];
    if (question.equation !== null) expressions.push(question.equation);
    for (const filter of question.filters) dependencies.add(filter.sourceId);
    for (const expression of expressions) {
      for (const variable of expression.variables) {
        const owner = fields.get(variableName(variable));
        if (owner !== undefined) dependencies.add(owner);
      }
    }
    for (const name of question.defaults?.variables ?? []) {
      const owner = fields.get(variableName(name));
      if (owner === undefined)
        question.issues.push("Variable inconnue dans une valeur par défaut.");
      else if (owner !== id) dependencies.add(owner);
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
    if (fields.size + question.cells.length > MAX_SURVEY_CELLS) {
      issues.push(`Aperçu limité à ${MAX_SURVEY_CELLS} cellules de réponse.`);
      return { study, groups, questions: new Map(), fields: new Map(), order: [], issues };
    }
    questions.set(source.id, question);
    for (const field of question.fields) {
      if (fields.has(field)) issues.push(`Cellule ambiguë : ${field}.`);
      fields.set(field, source.id);
    }
  }
  for (const question of questions.values()) {
    try {
      question.defaults = prepareDefaults(study, question);
    } catch (error) {
      question.issues.push(errorMessage(error));
    }
    prepareRules(
      question,
      groups.find((group) => group.id === question.source.groupId),
      fields,
      study,
    );
  }
  prepareFilters(questions);
  checkInactiveDefaultReferences(questions, fields);
  const order = sortDependencies(questions, fields);
  return { study, groups, questions, fields, order, issues };
}

export function answerLookup(answers: Answers): Lookup {
  return (name) => answers.get(variableName(name)) ?? "";
}

export interface EvaluationContext {
  language?: string;
  initialize?: ReadonlySet<string> | undefined;
  budget?: DefaultBudget;
}

export function evaluateSnapshot(
  runtime: SurveyRuntime,
  answers: Answers,
  context: EvaluationContext = {},
): Snapshot {
  const effective = new Map<string, string>();
  for (const [field, value] of answers) {
    if (runtime.fields.has(field)) effective.set(field, value);
  }
  const visible = new Set<string>();
  const relevant = new Set<string>();
  const activeFields = new Set<string>();
  const visibleSubquestions = new Set<string>();
  const visibleOther = new Set<string>();
  const activeChoices = new Map<string, ReadonlySet<string>>();
  const selections: SelectionCache = new Map();
  const issues = [...runtime.issues];
  const initialized = new Set<string>();
  const language =
    context.language ?? runtime.study.baseLanguage ?? runtime.study.languages[0] ?? "fr";
  const budget = context.budget ?? { remaining: 1_000_000 };
  for (const id of runtime.order) {
    const question = runtime.questions.get(id);
    if (question === undefined) continue;
    try {
      const isRelevant = question.relevance.every((expression) =>
        truth(expression.evaluate(answerLookup(effective))),
      );
      if (isRelevant && question.issues.length === 0) {
        if (context.initialize?.has(id) === true && question.defaults !== null)
          applyDefaults(question.defaults, effective, language, initialized, budget);
        relevant.add(id);
        if (attribute(question.source, "hidden") !== "1") visible.add(id);
        if (
          question.source.sourceType === "I" &&
          (effective.get(question.source.code) ?? "") === ""
        ) {
          effective.set(question.source.code, language);
        }
        const allowedChoices = applyChoiceFilters(question, effective, selections);
        if (allowedChoices !== null) activeChoices.set(id, allowedChoices);
        const activeRows = new Set<string>();
        for (const row of question.subquestions) {
          const rule = question.subquestionRules.get(row.id);
          const enabled = rule === undefined || truth(rule.evaluate(answerLookup(effective)));
          let filtered = false;
          if (row.scale === 0)
            filtered = !filterAllows(question.filters, row.code, effective, selections);
          if (enabled && !filtered) activeRows.add(row.id);
          if (enabled && (!filtered || attribute(question.source, "array_filter_style") === "1"))
            visibleSubquestions.add(row.id);
        }
        let otherActive = true;
        if (["M", "P"].includes(question.source.sourceType))
          otherActive = filterAllows(question.filters, "other", effective, selections);
        if (question.source.sourceType === "L" && allowedChoices !== null)
          otherActive = allowedChoices.has("-oth-");
        if (
          question.source.other &&
          (otherActive || attribute(question.source, "array_filter_style") === "1")
        )
          visibleOther.add(id);
        for (const [index, cell] of question.cells.entries()) {
          let active =
            (cell.rowId === null || activeRows.has(cell.rowId)) &&
            (cell.columnId === null || activeRows.has(cell.columnId));
          if (question.source.sourceType === "R" && allowedChoices !== null)
            active = active && index < allowedChoices.size;
          if (
            ["M", "P"].includes(question.source.sourceType) &&
            (cell.role === "other" || cell.key === `${question.source.code}_othercomment`)
          )
            active = active && otherActive;
          if (cell.role === "other" && ["L", "!"].includes(question.source.sourceType))
            active = active && effective.get(question.source.code) === "-oth-";
          if (active) activeFields.add(cell.key);
          else effective.delete(cell.key);
        }
        preserveExistingExclusiveSelection(question, effective, activeFields, initialized);
        applyExclusive(question, effective, activeFields);
        applyComments(question, effective, activeFields);
        if (question.equation !== null) {
          effective.set(
            question.source.code,
            String(question.equation.evaluate(answerLookup(effective)) ?? ""),
          );
        }
      } else {
        if (question.issues.length > 0) visible.add(id);
        for (const field of question.fields) effective.delete(field);
      }
    } catch (error) {
      issues.push(`${question.source.code} : ${errorMessage(error)}`);
      for (const field of question.fields) effective.delete(field);
      relevant.delete(id);
      visible.add(id);
    }
  }
  const groups = runtime.groups.filter((group) =>
    [...runtime.questions.values()].some(
      (question) => question.source.groupId === group.id && visible.has(question.source.id),
    ),
  );
  for (const key of initialized) {
    if (!effective.has(key)) initialized.delete(key);
  }
  return {
    answers: effective,
    visible,
    relevant,
    activeFields,
    visibleSubquestions,
    visibleOther,
    activeChoices,
    groups,
    issues,
    initialized,
  };
}

export function runtimeIssues(runtime: SurveyRuntime, snapshot: Snapshot): string[] {
  const issues = [...snapshot.issues];
  for (const question of runtime.questions.values()) {
    for (const issue of question.issues) issues.push(`${question.source.code} : ${issue}`);
  }
  return issues;
}
