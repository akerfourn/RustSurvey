import { attribute } from "./attributes.ts";
import { MAX_QUESTION_CELLS } from "./cells.ts";
import type { SurveyQuestion } from "./types.ts";

export interface NumberRange {
  min: number;
  max: number;
  step: number;
}

export function matrixRange(question: SurveyQuestion): NumberRange {
  const settings: NumberRange = { min: 1, max: 10, step: 1 };
  for (const name of ["min", "max", "step"] as const) {
    const value = attribute(question, `multiflexible_${name}`);
    if (value === "") continue;
    const number = Number(value);
    if (!Number.isFinite(number) || value.trim() === "")
      throw new Error("Borne de matrice invalide.");
    settings[name] = number;
  }
  if (settings.step < 1e-10 || settings.max < settings.min)
    throw new Error("Intervalle de matrice invalide.");
  return settings;
}

export function matrixOptions(question: SurveyQuestion, cellCount: number): string[] {
  const { min, max, step } = matrixRange(question);
  const count = Math.floor((max - min) / step + 1e-9) + 1;
  if (!Number.isSafeInteger(count) || count * cellCount > MAX_QUESTION_CELLS) {
    throw new Error(`Matrice limitée à ${MAX_QUESTION_CELLS} choix affichés.`);
  }
  return Array.from({ length: count }, (_, index) =>
    String(Number((min + index * step).toPrecision(12))),
  );
}

export function matrixInputBounds(question: SurveyQuestion): { min?: number; max?: number } {
  const result: { min?: number; max?: number } = {};
  for (const name of ["min", "max"] as const) {
    const value = attribute(question, `multiflexible_${name}`);
    if (value === "") continue;
    const number = Number(value);
    if (!Number.isFinite(number) || value.trim() === "")
      throw new Error("Borne de matrice invalide.");
    result[name] = number;
  }
  if (result.min !== undefined && result.max !== undefined && result.min > result.max)
    throw new Error("Intervalle de matrice invalide.");
  return result;
}
