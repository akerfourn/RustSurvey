import type { SurveyStudy } from "../survey/types.ts";
import { QUESTION_PROFILES, questionProfile } from "./question-profile.ts";

export const RENDER_TYPES = {
  short_text: "S",
  long_text: "T",
  huge_text: "U",
  number: "N",
  date: "D",
  single_choice: "L",
  dropdown: "!",
  multiple_choice: "M",
  yes_no: "Y",
  information: "X",
  rating_five: "5",
  gender: "G",
  single_choice_comment: "O",
  multiple_choice_comment: "P",
  multiple_text: "Q",
  multiple_numbers: "K",
  array_five: "A",
  array_ten: "B",
  array_yes_no: "C",
  array_increase: "E",
  array_choice: "F",
  array_column: "H",
  dual_scale: "1",
  number_matrix: ":",
  text_matrix: ";",
  ranking: "R",
  language: "I",
  equation: "*",
  file_upload: "|",
  unsupported: "?",
} as const;
export type QuestionKind = keyof typeof RENDER_TYPES;
export interface SourceReference {
  id: string;
  kind: string | null;
}
export interface Translation {
  language: string;
  text: string | null;
  help: string | null;
}
export interface SurveyTranslation {
  language: string;
  title: string | null;
  welcome: string | null;
  end: string | null;
}
export type Mandatory = "no" | "soft" | "strict";
export interface Choice {
  id: string;
  code: string;
  label: string;
  help: string;
  scale: number;
  relevance: string;
  translations: Translation[];
  source: SourceReference | null;
}
export interface Question {
  id: string;
  code: string;
  title: string;
  help: string;
  kind: QuestionKind;
  mandatory: Mandatory;
  choices: Choice[];
  subquestions: Choice[];
  relevance: string;
  translations: Translation[];
  attributes: import("../survey/types.ts").QuestionAttribute[];
  other: boolean;
  validation: string;
  randomizationGroup: string;
  theme: string;
  source: SourceReference | null;
  media: QuestionMedia[];
}
export interface QuestionMedia {
  id: string;
  alt: string;
  language: string | null;
}
export interface Group {
  id: string;
  title: string;
  description: string;
  questions: Question[];
  relevance: string;
  translations: Translation[];
  randomizationGroup: string;
  source: SourceReference | null;
}
export interface Definition {
  schemaVersion: number;
  title: string;
  language: string;
  welcome: string;
  end: string;
  groups: Group[];
  languages: string[];
  translations: SurveyTranslation[];
  showWelcome: boolean | null;
}
export interface StudySummary {
  id: string;
  title: string;
  revision: number;
  createdAt: number;
  updatedAt: number;
}
export interface StoredStudy {
  id: string;
  revision: number;
  createdAt: number;
  updatedAt: number;
  definition: Definition;
  source: ImportInfo | null;
}
export interface ImportInfo {
  sha256: string;
  report: import("../survey/types.ts").InspectionReport;
}
export interface Session {
  authenticated: boolean;
  configured: boolean;
  username: string | null;
  csrfToken: string | null;
}
export interface ValidationIssue {
  path: string;
  message: string;
}

export const QUESTION_KINDS: ReadonlyMap<QuestionKind, string> = new Map(
  [...QUESTION_PROFILES].map(([kind, profile]) => [kind, profile.label]),
);

export function createId(): string {
  return crypto.randomUUID();
}
export function createGroup(makeId: () => string = createId): Group {
  return {
    id: makeId(),
    title: "Nouveau groupe",
    description: "",
    questions: [],
    relevance: "1",
    translations: [],
    randomizationGroup: "",
    source: null,
  };
}
export function createDefinition(makeId: () => string = createId): Definition {
  return {
    schemaVersion: 3,
    languages: [],
    translations: [],
    showWelcome: null,
    title: "Nouveau questionnaire",
    language: "fr",
    welcome: "",
    end: "Merci pour votre participation.",
    groups: [createGroup(makeId)],
  };
}
export function createChoice(code: string, label: string, makeId: () => string = createId): Choice {
  return {
    id: makeId(),
    code,
    label,
    help: "",
    scale: 0,
    relevance: "1",
    translations: [],
    source: null,
  };
}
export function createChoices(makeId: () => string = createId): Choice[] {
  return [createChoice("A1", "Premier choix", makeId), createChoice("A2", "Second choix", makeId)];
}
export function createQuestion(definition: Definition, makeId: () => string = createId): Question {
  const codes = new Set(
    definition.groups.flatMap((group) => group.questions.map((question) => question.code)),
  );
  let number = 1;
  while (codes.has(`Q${number}`)) number += 1;
  return {
    id: makeId(),
    code: `Q${number}`,
    title: "Nouvelle question",
    help: "",
    kind: "short_text",
    mandatory: "no",
    choices: [],
    subquestions: [],
    relevance: "1",
    translations: [],
    attributes: [],
    other: false,
    validation: "",
    randomizationGroup: "",
    theme: "core",
    source: null,
    media: [],
  };
}
export function changeKind(
  question: Question,
  kind: QuestionKind,
  makeId: () => string = createId,
): Question {
  if (kind === question.kind) return question;
  const profile = questionProfile(kind);
  const oldProfile = questionProfile(question.kind);
  let choices = question.choices.filter((choice) => choice.scale === 0);
  let rows = question.subquestions.filter((row) => row.scale === 0);
  if (profile.choices && !profile.rows && choices.length === 0 && !oldProfile.choices)
    choices = rows;
  if (profile.rows && !profile.choices && rows.length === 0 && !oldProfile.rows)
    rows = question.choices;
  if (profile.choices && choices.length === 0) choices = createChoices(makeId);
  if (profile.rows && rows.length === 0) rows = createChoices(makeId);
  if (!profile.choices) choices = [];
  let subquestions: Choice[] = [];
  if (profile.rows) subquestions = rows;
  if (profile.columns) {
    let columns = question.subquestions.filter((row) => row.scale === 1);
    if (columns.length === 0)
      columns = createChoices(makeId).map((item) => ({ ...item, scale: 1 }));
    subquestions = [...subquestions, ...columns];
  }
  if (profile.dualScale) {
    let secondScale = question.choices.filter((choice) => choice.scale === 1);
    if (secondScale.length === 0)
      secondScale = createChoices(makeId).map((choice) => ({ ...choice, scale: 1 }));
    choices = [...choices, ...secondScale];
  }
  let mandatory = question.mandatory;
  if (kind === "information" || kind === "equation") mandatory = "no";
  const attributes = question.attributes.filter((item) => !isTypeSpecificAttribute(item.name));
  if (kind === "equation") attributes.push({ name: "equation", value: "0", language: null });
  return {
    ...question,
    kind,
    choices,
    subquestions,
    mandatory,
    attributes,
    other: profile.other && question.other,
  };
}

function isTypeSpecificAttribute(name: string): boolean {
  return (
    name === "equation" ||
    name === "commented_checkbox" ||
    name.startsWith("slider_") ||
    name.startsWith("multiflexible_") ||
    name === "input_boxes"
  );
}
export function moveItem<T>(items: readonly T[], index: number, direction: -1 | 1): T[] {
  const next = index + direction;
  if (index < 0 || index >= items.length || next < 0 || next >= items.length) return [...items];
  const result = [...items];
  [result[index], result[next]] = [result[next]!, result[index]!];
  return result;
}
function labels(translations: Translation[], language: string, text: string, help: string) {
  const result = translations.map((item) => ({ ...item }));
  const index = result.findIndex((item) => item.language === language);
  const base = { language, text, help };
  if (index < 0) result.push(base);
  else result[index] = base;
  return result;
}
export function toViewer(definition: Definition, id: string | null = null): SurveyStudy {
  const language = definition.language;
  const translations = definition.translations.filter((item) => item.language !== language);
  const study: SurveyStudy = {
    surveyId: id,
    baseLanguage: language,
    languages: definition.languages.length === 0 ? [language] : definition.languages,
    titles: [
      { language, value: definition.title },
      ...translations.map((item) => ({ language: item.language, value: item.title })),
    ],
    messages: [
      { language, welcome: definition.welcome, end: definition.end },
      ...translations.map((item) => ({
        language: item.language,
        welcome: item.welcome,
        end: item.end,
      })),
    ],
    showWelcome: definition.showWelcome ?? definition.welcome !== "",
    groups: [],
    questions: [],
    subquestions: [],
    answers: [],
  };
  for (const [order, group] of definition.groups.entries()) {
    study.groups.push({
      id: group.id,
      order,
      relevance: group.relevance,
      randomizationGroup: group.randomizationGroup,
      labels: labels(group.translations, language, group.title, group.description).map((item) => ({
        language: item.language,
        name: item.text,
        description: item.help,
      })),
    });
    for (const [order, question] of group.questions.entries()) {
      let sourceType: string = RENDER_TYPES[question.kind];
      if (question.kind === "unsupported") sourceType = question.source?.kind ?? "?";
      study.questions.push({
        id: question.id,
        groupId: group.id,
        code: question.code,
        sourceType,
        theme: question.theme,
        family: "native",
        mandatory: question.mandatory,
        order,
        relevance: question.relevance,
        randomizationGroup: question.randomizationGroup,
        validation: question.validation,
        other: question.other,
        labels: labels(question.translations, language, question.title, question.help),
        attributes: question.attributes,
        media: question.media,
      });
      for (const [order, row] of question.subquestions.entries())
        study.subquestions.push({
          id: row.id,
          parentId: question.id,
          groupId: group.id,
          code: row.code,
          scale: row.scale,
          order,
          relevance: row.relevance,
          labels: labels(row.translations, language, row.label, row.help),
        });
      for (const [order, choice] of question.choices.entries())
        study.answers.push({
          id: choice.id,
          questionId: question.id,
          code: choice.code,
          scale: choice.scale,
          order,
          labels: labels(choice.translations, language, choice.label, choice.help).map((item) => ({
            language: item.language,
            text: item.text,
          })),
        });
    }
  }
  return study;
}
