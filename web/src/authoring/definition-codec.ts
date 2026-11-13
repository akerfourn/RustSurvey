import {
  RENDER_TYPES,
  type Choice,
  type Definition,
  type Group,
  type ImportInfo,
  type Mandatory,
  type Question,
  type QuestionKind,
  type SourceReference,
  type SurveyTranslation,
  type Translation,
} from "./model.ts";
import { decodeReport } from "../api/inspection.ts";

function object(value: unknown): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value))
    throw new Error("Définition du serveur invalide.");
  return value as Record<string, unknown>;
}
function text(value: unknown): string {
  if (typeof value !== "string") throw new Error("Texte du serveur invalide.");
  return value;
}
function nullable(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  return text(value);
}
function array<T>(value: unknown, decode: (item: unknown) => T): T[] {
  if (value === undefined) return [];
  if (!Array.isArray(value)) throw new Error("Liste du serveur invalide.");
  return value.map(decode);
}
function number(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value))
    throw new Error("Nombre du serveur invalide.");
  return value;
}
function bool(value: unknown): boolean {
  if (typeof value !== "boolean") throw new Error("Valeur du serveur invalide.");
  return value;
}
function source(value: unknown): SourceReference | null {
  if (value === null || value === undefined) return null;
  const item = object(value);
  return { id: text(item.id), kind: nullable(item.kind) };
}
function translation(value: unknown): Translation {
  const item = object(value);
  return { language: text(item.language), text: nullable(item.text), help: nullable(item.help) };
}
function surveyTranslation(value: unknown): SurveyTranslation {
  const item = object(value);
  return {
    language: text(item.language),
    title: nullable(item.title),
    welcome: nullable(item.welcome),
    end: nullable(item.end),
  };
}
function choice(value: unknown): Choice {
  const item = object(value);
  return {
    id: text(item.id),
    code: text(item.code),
    label: text(item.label),
    help: text(item.help ?? ""),
    scale: number(item.scale ?? 0),
    relevance: text(item.relevance ?? "1"),
    translations: array(item.translations, translation),
    source: source(item.source),
  };
}
function question(value: unknown, legacy: boolean): Question {
  const item = object(value);
  const kind = text(item.kind);
  if (!Object.hasOwn(RENDER_TYPES, kind)) throw new Error("Type de question inconnu.");
  const mandatory = text(item.mandatory);
  if (!["no", "soft", "strict"].includes(mandatory))
    throw new Error("Caractère obligatoire inconnu.");
  let choices = array(item.choices, choice);
  let subquestions = array(item.subquestions, choice);
  if (legacy && kind === "multiple_choice") {
    if (subquestions.length > 0 && choices.length > 0) throw new Error("Définition v1 ambiguë.");
    if (subquestions.length === 0) subquestions = choices;
    choices = [];
  }
  return {
    id: text(item.id),
    code: text(item.code),
    title: text(item.title),
    help: text(item.help),
    kind: kind as QuestionKind,
    mandatory: mandatory as Mandatory,
    choices,
    subquestions,
    relevance: text(item.relevance ?? "1"),
    translations: array(item.translations, translation),
    other: bool(item.other ?? false),
    validation: text(item.validation ?? ""),
    randomizationGroup: text(item.randomizationGroup ?? ""),
    theme: text(item.theme ?? ""),
    source: source(item.source),
    attributes: array(item.attributes, (value) => {
      const attribute = object(value);
      return {
        name: text(attribute.name),
        value: text(attribute.value),
        language: nullable(attribute.language),
      };
    }),
    media: array(item.media, (value) => {
      const image = object(value);
      return { id: text(image.id), alt: text(image.alt), language: nullable(image.language) };
    }),
  };
}
function group(value: unknown, legacy: boolean): Group {
  const item = object(value);
  return {
    id: text(item.id),
    title: text(item.title),
    description: text(item.description),
    questions: array(item.questions, (item) => question(item, legacy)),
    relevance: text(item.relevance ?? "1"),
    translations: array(item.translations, translation),
    randomizationGroup: text(item.randomizationGroup ?? ""),
    source: source(item.source),
  };
}
export function decodeDefinition(value: unknown): Definition {
  const item = object(value);
  if (item.schemaVersion !== 1 && item.schemaVersion !== 2 && item.schemaVersion !== 3)
    throw new Error("Version de questionnaire non prise en charge.");
  const definition: Definition = {
    schemaVersion: 3,
    title: text(item.title),
    language: text(item.language),
    welcome: text(item.welcome),
    end: text(item.end),
    groups: array(item.groups, (value) => group(value, item.schemaVersion === 1)),
    languages: array(item.languages, text),
    translations: array(item.translations, surveyTranslation),
    showWelcome:
      item.showWelcome === null || item.showWelcome === undefined ? null : bool(item.showWelcome),
  };
  if (
    item.schemaVersion !== 3 &&
    definition.groups.some((group) => group.questions.some((question) => question.media.length > 0))
  )
    throw new Error("Les références de médias nécessitent le format natif v3.");
  return definition;
}
export function decodeSourceInfo(value: unknown): ImportInfo | null {
  if (value === null || value === undefined) return null;
  const item = object(value);
  return { sha256: text(item.sha256), report: decodeReport(item.report) };
}
