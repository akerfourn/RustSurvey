import { defaultTargetKey, type DefaultCoordinate, type DefaultValue } from "../survey/defaults.ts";
import type { Question } from "./model.ts";

export interface DefaultTarget {
  key: string;
  coordinate: DefaultCoordinate;
  role: DefaultValue["role"];
  label: string;
}

export function defaultTargets(question: Question): DefaultTarget[] {
  const targets: DefaultTarget[] = [];
  if (
    [
      "short_text",
      "long_text",
      "huge_text",
      "number",
      "date",
      "single_choice",
      "dropdown",
      "rating_five",
      "yes_no",
      "gender",
      "single_choice_comment",
    ].includes(question.kind)
  ) {
    targets.push({
      key: "answer",
      coordinate: { kind: "primary" },
      role: "answer",
      label: "Réponse principale",
    });
    if (question.other && ["single_choice", "dropdown"].includes(question.kind))
      targets.push({
        key: "other",
        coordinate: { kind: "primary" },
        role: "other",
        label: "Texte de la réponse Autre",
      });
  }
  if (
    [
      "multiple_choice",
      "multiple_choice_comment",
      "multiple_text",
      "multiple_numbers",
      "array_five",
      "array_ten",
      "array_yes_no",
      "array_increase",
      "array_choice",
      "array_column",
    ].includes(question.kind)
  ) {
    for (const row of question.subquestions.filter((row) => row.scale === 0)) {
      const coordinate: DefaultCoordinate = { kind: "subquestion", id: row.id, scale: null };
      targets.push({
        key: defaultTargetKey(coordinate, "answer"),
        coordinate,
        role: "answer",
        label: `${row.code} — ${row.label}`,
      });
    }
  }
  return targets;
}

export function setDefaultValue(
  question: Question,
  target: DefaultTarget,
  language: string,
  value: string | null,
): Question {
  const defaults = question.defaults ?? { sameForAllLanguages: false, values: [] };
  const values = defaults.values.filter(
    (entry) =>
      entry.language !== language || defaultTargetKey(entry.coordinate, entry.role) !== target.key,
  );
  if (value !== null)
    values.push({ coordinate: target.coordinate, role: target.role, language, value });
  return { ...question, defaults: { ...defaults, values } };
}

export function pruneQuestionDefaults(question: Question, previous: Question): Question {
  if (question.defaults === null) return question;
  const keys = new Set(defaultTargets(question).map((target) => target.key));
  const previousKeys = new Set(defaultTargets(previous).map((target) => target.key));
  const values = question.defaults.values.filter(
    (value) =>
      keys.has(defaultTargetKey(value.coordinate, value.role)) ||
      !previousKeys.has(defaultTargetKey(value.coordinate, value.role)),
  );
  if (values.length === question.defaults.values.length) return question;
  return { ...question, defaults: { ...question.defaults, values } };
}
