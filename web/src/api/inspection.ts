import type {
  ChoiceLabel,
  Diagnostic,
  GroupLabel,
  Inspection,
  LocalizedTitle,
  QuestionAttribute,
  QuestionLabel,
  SurveyChoice,
  SurveyGroup,
  SurveyMessages,
  SurveyQuestion,
  SurveyStudy,
  SurveySubquestion,
} from "../survey/types.ts";

export const MAX_UPLOAD_BYTES = 8 * 1024 * 1024;

function record(value: unknown): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value))
    throw new Error("Réponse inattendue du serveur.");
  return value as Record<string, unknown>;
}

function string(value: unknown): string {
  if (typeof value !== "string") throw new Error("Texte manquant dans la réponse du serveur.");
  return value;
}

function nullable(value: unknown): string | null {
  if (value === null) return null;
  return string(value);
}

function number(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw new Error("Compteur invalide dans la réponse du serveur.");
  return value;
}

function boolean(value: unknown): boolean {
  if (typeof value !== "boolean") throw new Error("Valeur invalide dans la réponse du serveur.");
  return value;
}

function array<T>(value: unknown, decode: (item: unknown) => T): T[] {
  if (!Array.isArray(value)) throw new Error("Liste manquante dans la réponse du serveur.");
  return value.map(decode);
}

function title(value: unknown): LocalizedTitle {
  const item = record(value);
  return { language: string(item.language), value: nullable(item.value) };
}

function questionLabel(value: unknown): QuestionLabel {
  const item = record(value);
  return { language: string(item.language), text: nullable(item.text), help: nullable(item.help) };
}

function surveyMessages(value: unknown): SurveyMessages {
  const item = record(value);
  return {
    language: string(item.language),
    welcome: nullable(item.welcome),
    end: nullable(item.end),
  };
}

function groupLabel(value: unknown): GroupLabel {
  const item = record(value);
  return {
    language: string(item.language),
    name: nullable(item.name),
    description: nullable(item.description),
  };
}

function choiceLabel(value: unknown): ChoiceLabel {
  const item = record(value);
  return { language: string(item.language), text: nullable(item.text) };
}

function attribute(value: unknown): QuestionAttribute {
  const item = record(value);
  return { name: string(item.name), value: string(item.value), language: nullable(item.language) };
}

function group(value: unknown): SurveyGroup {
  const item = record(value);
  return {
    id: string(item.id),
    order: number(item.order),
    relevance: string(item.relevance),
    randomizationGroup: string(item.randomizationGroup),
    labels: array(item.labels, groupLabel),
  };
}

function question(value: unknown): SurveyQuestion {
  const item = record(value);
  const mandatory = item.mandatory;
  if (mandatory !== "no" && mandatory !== "soft" && mandatory !== "strict")
    throw new Error("Contrainte obligatoire inconnue.");
  return {
    id: string(item.id),
    groupId: string(item.groupId),
    code: string(item.code),
    sourceType: string(item.sourceType),
    theme: string(item.theme),
    family: string(item.family),
    mandatory,
    order: number(item.order),
    relevance: string(item.relevance),
    randomizationGroup: string(item.randomizationGroup),
    validation: string(item.validation),
    other: boolean(item.other),
    labels: array(item.labels, questionLabel),
    attributes: array(item.attributes, attribute),
  };
}

function subquestion(value: unknown): SurveySubquestion {
  const item = record(value);
  return {
    id: string(item.id),
    parentId: string(item.parentId),
    groupId: string(item.groupId),
    code: string(item.code),
    scale: number(item.scale),
    order: number(item.order),
    relevance: string(item.relevance),
    labels: array(item.labels, questionLabel),
  };
}

function choice(value: unknown): SurveyChoice {
  const item = record(value);
  return {
    id: string(item.id),
    questionId: string(item.questionId),
    code: string(item.code),
    order: number(item.order),
    scale: number(item.scale),
    labels: array(item.labels, choiceLabel),
  };
}

function study(value: unknown): SurveyStudy {
  const item = record(value);
  return {
    surveyId: nullable(item.surveyId),
    baseLanguage: nullable(item.baseLanguage),
    languages: array(item.languages, string),
    titles: array(item.titles, title),
    showWelcome: boolean(item.showWelcome),
    messages: array(item.messages, surveyMessages),
    groups: array(item.groups, group),
    questions: array(item.questions, question),
    subquestions: array(item.subquestions, subquestion),
    answers: array(item.answers, choice),
  };
}

function diagnostic(value: unknown): Diagnostic {
  const item = record(value);
  const severity = item.severity;
  if (severity !== "error" && severity !== "warning") throw new Error("Diagnostic inconnu.");
  return {
    severity,
    code: string(item.code),
    location: string(item.location),
    message: string(item.message),
  };
}

export function decodeInspection(value: unknown): Inspection {
  const result = record(value);
  const metadata = record(result.metadata);
  const report = record(result.report);
  let survey: SurveyStudy | null = null;
  if (result.study !== null) survey = study(result.study);
  return {
    metadata: {
      surveyId: nullable(metadata.surveyId),
      baseLanguage: nullable(metadata.baseLanguage),
      languages: array(metadata.languages, string),
      titles: array(metadata.titles, title),
    },
    study: survey,
    report: {
      sourceSchema: number(report.sourceSchema),
      targetSchema: number(report.targetSchema),
      errors: number(report.errors),
      warnings: number(report.warnings),
      omittedDiagnostics: number(report.omittedDiagnostics),
      checkedSections: array(report.checkedSections, string),
      diagnostics: array(report.diagnostics, diagnostic),
      sections: array(report.sections, (value) => {
        const section = record(value);
        return { name: string(section.name), rows: number(section.rows) };
      }),
    },
  };
}

const ERROR_MESSAGES = new Map([
  ["invalid_lss", "Ce fichier n’est pas un LSS XML lisible."],
  ["upload_too_large", "Le fichier dépasse la limite de 8 Mio."],
  ["inspection_busy", "Deux analyses sont déjà en cours. Réessayez dans un instant."],
  ["upload_timeout", "Le transfert a dépassé le délai autorisé. Réessayez."],
]);

export async function inspectFile(file: File, signal: AbortSignal): Promise<Inspection> {
  if (file.size === 0) throw new Error("Le fichier sélectionné est vide.");
  if (file.size > MAX_UPLOAD_BYTES) throw new Error("Le fichier dépasse la limite de 8 Mio.");
  const response = await fetch("/api/surveys/inspect", {
    method: "POST",
    headers: { "Content-Type": "application/xml" },
    body: file,
    signal,
    cache: "no-store",
  });
  const body: unknown = await response.json();
  if (!response.ok) {
    const details = record(record(body).error);
    const code = string(details.code);
    throw new Error(ERROR_MESSAGES.get(code) ?? `L’analyse a échoué (HTTP ${response.status}).`);
  }
  return decodeInspection(body);
}

export async function loadExample(
  signal: AbortSignal,
  variant: "example" | "controls" = "example",
): Promise<File> {
  const response = await fetch(`/api/surveys/${variant}.lss`, { signal, cache: "no-store" });
  if (!response.ok)
    throw new Error("L’exemple n’est pas disponible. Vérifiez que le backend est démarré.");
  return new File([await response.blob()], `rustsurvey-${variant}.lss`, {
    type: "application/xml",
  });
}
