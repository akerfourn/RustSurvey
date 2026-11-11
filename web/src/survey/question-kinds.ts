import type { SurveyChoice } from "./types.ts";

export interface ChoiceOption {
  code: string;
  label: string;
}

export function isChoiceArray(type: string): boolean {
  return ["A", "B", "C", "E", "F", "H", "1"].includes(type);
}

export function isMatrix(type: string): boolean {
  return type === ":" || type === ";";
}

export function fixedOptions(type: string, language = "fr"): ChoiceOption[] | null {
  if (["5", "A", "B"].includes(type)) {
    let count = 5;
    if (type === "B") count = 10;
    return Array.from({ length: count }, (_, index) => ({
      code: String(index + 1),
      label: String(index + 1),
    }));
  }
  const french = language === "fr" || language.startsWith("fr-");
  if (type === "Y" || type === "C") {
    const options = [{ code: "Y", label: french ? "Oui" : "Yes" }];
    if (type === "C") options.push({ code: "U", label: french ? "Incertain" : "Uncertain" });
    options.push({ code: "N", label: french ? "Non" : "No" });
    return options;
  }
  if (type === "G")
    return [
      { code: "F", label: french ? "Féminin" : "Female" },
      { code: "M", label: french ? "Masculin" : "Male" },
    ];
  if (type === "E")
    return [
      { code: "I", label: french ? "Augmenter" : "Increase" },
      { code: "S", label: french ? "Identique" : "Same" },
      { code: "D", label: french ? "Diminuer" : "Decrease" },
    ];
  return null;
}

export function choiceCodes(type: string, choices: readonly SurveyChoice[], scale = 0): string[] {
  const fixed = fixedOptions(type);
  if (fixed !== null) return fixed.map((option) => option.code);
  return choices.filter((choice) => choice.scale === scale).map((choice) => choice.code);
}

const BASE_THEMES: ReadonlyMap<string, string> = new Map([
  ["!", "list_dropdown"],
  ["*", "equation"],
  ["1", "arrays/dualscale"],
  ["5", "5pointchoice"],
  [":", "arrays/multiflexi"],
  [";", "arrays/texts"],
  ["A", "arrays/5point"],
  ["B", "arrays/10point"],
  ["C", "arrays/yesnouncertain"],
  ["D", "date"],
  ["E", "arrays/increasesamedecrease"],
  ["F", "arrays/array"],
  ["G", "gender"],
  ["H", "arrays/column"],
  ["I", "language"],
  ["K", "multiplenumeric"],
  ["L", "listradio"],
  ["M", "multiplechoice"],
  ["N", "numerical"],
  ["O", "list_with_comment"],
  ["P", "multiplechoice_with_comments"],
  ["Q", "multipleshorttext"],
  ["R", "ranking"],
  ["S", "shortfreetext"],
  ["T", "longfreetext"],
  ["U", "hugefreetext"],
  ["X", "boilerplate"],
  ["Y", "yesno"],
  ["|", "file_upload"],
]);

export function isBaseTheme(type: string, theme: string | undefined): boolean {
  return theme === undefined || theme === "" || theme === "core" || BASE_THEMES.get(type) === theme;
}
