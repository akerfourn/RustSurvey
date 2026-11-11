import type { SurveyStudy } from "../survey/types.ts";

export type QuestionKind =
  | "short_text"
  | "long_text"
  | "number"
  | "date"
  | "single_choice"
  | "multiple_choice"
  | "yes_no"
  | "information";
export type Mandatory = "no" | "soft" | "strict";
export interface Choice {
  id: string;
  code: string;
  label: string;
}
export interface Question {
  id: string;
  code: string;
  title: string;
  help: string;
  kind: QuestionKind;
  mandatory: Mandatory;
  choices: Choice[];
}
export interface Group {
  id: string;
  title: string;
  description: string;
  questions: Question[];
}
export interface Definition {
  schemaVersion: number;
  title: string;
  language: string;
  welcome: string;
  end: string;
  groups: Group[];
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

export const QUESTION_KINDS: ReadonlyMap<QuestionKind, string> = new Map([
  ["short_text", "Texte court"],
  ["long_text", "Texte long"],
  ["number", "Nombre"],
  ["date", "Date"],
  ["single_choice", "Choix unique"],
  ["multiple_choice", "Choix multiples"],
  ["yes_no", "Oui / non"],
  ["information", "Information"],
]);
const SOURCE_TYPES: Record<QuestionKind, string> = {
  short_text: "S",
  long_text: "T",
  number: "N",
  date: "D",
  single_choice: "L",
  multiple_choice: "M",
  yes_no: "Y",
  information: "X",
};

export function createId(): string {
  return crypto.randomUUID();
}
export function createGroup(makeId: () => string = createId): Group {
  return { id: makeId(), title: "Nouveau groupe", description: "", questions: [] };
}
export function createDefinition(makeId: () => string = createId): Definition {
  return {
    schemaVersion: 1,
    title: "Nouveau questionnaire",
    language: "fr",
    welcome: "",
    end: "Merci pour votre participation.",
    groups: [createGroup(makeId)],
  };
}
export function createChoices(makeId: () => string = createId): Choice[] {
  return [
    { id: makeId(), code: "A1", label: "Premier choix" },
    { id: makeId(), code: "A2", label: "Second choix" },
  ];
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
  };
}
export function changeKind(
  question: Question,
  kind: QuestionKind,
  makeId: () => string = createId,
): Question {
  let choices: Choice[] = [];
  if (kind === "single_choice" || kind === "multiple_choice")
    choices = question.choices.length > 0 ? question.choices : createChoices(makeId);
  let mandatory = question.mandatory;
  if (kind === "information") mandatory = "no";
  return { ...question, kind, choices, mandatory };
}
export function moveItem<T>(items: readonly T[], index: number, direction: -1 | 1): T[] {
  const next = index + direction;
  if (index < 0 || index >= items.length || next < 0 || next >= items.length) return [...items];
  const result = [...items];
  [result[index], result[next]] = [result[next]!, result[index]!];
  return result;
}
export function toViewer(definition: Definition, id: string | null = null): SurveyStudy {
  const language = definition.language;
  const study: SurveyStudy = {
    surveyId: id,
    baseLanguage: language,
    languages: [language],
    titles: [{ language, value: definition.title }],
    showWelcome: definition.welcome !== "",
    messages: [{ language, welcome: definition.welcome, end: definition.end }],
    groups: [],
    questions: [],
    subquestions: [],
    answers: [],
  };
  for (const [order, group] of definition.groups.entries()) {
    study.groups.push({
      id: group.id,
      order,
      relevance: "1",
      randomizationGroup: "",
      labels: [{ language, name: group.title, description: group.description }],
    });
    for (const [order, question] of group.questions.entries()) {
      study.questions.push({
        id: question.id,
        groupId: group.id,
        code: question.code,
        sourceType: SOURCE_TYPES[question.kind],
        theme: "core",
        family: "native",
        mandatory: question.mandatory,
        order,
        relevance: "1",
        randomizationGroup: "",
        validation: "",
        other: false,
        labels: [{ language, text: question.title, help: question.help }],
        attributes: [],
      });
      for (const [order, choice] of question.choices.entries()) {
        if (question.kind === "multiple_choice")
          study.subquestions.push({
            id: choice.id,
            parentId: question.id,
            groupId: group.id,
            code: choice.code,
            scale: 0,
            order,
            relevance: "1",
            labels: [{ language, text: choice.label, help: "" }],
          });
        else
          study.answers.push({
            id: choice.id,
            questionId: question.id,
            code: choice.code,
            order,
            scale: 0,
            labels: [{ language, text: choice.label }],
          });
      }
    }
  }
  return study;
}
