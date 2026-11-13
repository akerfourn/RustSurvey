import type { QuestionKind } from "./model.ts";

export interface QuestionProfile {
  label: string;
  choices: boolean;
  rows: boolean;
  columns: boolean;
  dualScale: boolean;
  other: boolean;
}

function profile(
  label: string,
  options: Partial<Omit<QuestionProfile, "label">> = {},
): QuestionProfile {
  return {
    label,
    choices: false,
    rows: false,
    columns: false,
    dualScale: false,
    other: false,
    ...options,
  };
}

export const QUESTION_PROFILES: ReadonlyMap<QuestionKind, QuestionProfile> = new Map([
  ["short_text", profile("Texte court")],
  ["long_text", profile("Texte long")],
  ["number", profile("Nombre")],
  ["date", profile("Date")],
  ["single_choice", profile("Choix unique", { choices: true, other: true })],
  ["multiple_choice", profile("Choix multiples", { rows: true, other: true })],
  ["yes_no", profile("Oui / non")],
  ["information", profile("Information")],
  ["huge_text", profile("Texte très long")],
  ["dropdown", profile("Liste déroulante", { choices: true, other: true })],
  ["rating_five", profile("Note de 1 à 5")],
  ["gender", profile("Genre")],
  ["single_choice_comment", profile("Choix unique avec commentaire", { choices: true })],
  [
    "multiple_choice_comment",
    profile("Choix multiples avec commentaires", { rows: true, other: true }),
  ],
  ["multiple_text", profile("Textes multiples", { rows: true })],
  ["multiple_numbers", profile("Nombres multiples", { rows: true })],
  ["array_five", profile("Tableau : notes de 1 à 5", { rows: true })],
  ["array_ten", profile("Tableau : notes de 1 à 10", { rows: true })],
  ["array_yes_no", profile("Tableau : oui / non / incertain", { rows: true })],
  ["array_increase", profile("Tableau : augmentation / identique / diminution", { rows: true })],
  ["array_choice", profile("Tableau : choix personnalisés", { rows: true, choices: true })],
  ["array_column", profile("Tableau : choix par colonne", { rows: true, choices: true })],
  [
    "dual_scale",
    profile("Tableau : double échelle", { rows: true, choices: true, dualScale: true }),
  ],
  ["number_matrix", profile("Matrice de nombres ou de cases", { rows: true, columns: true })],
  ["text_matrix", profile("Matrice de textes", { rows: true, columns: true })],
  ["ranking", profile("Classement", { choices: true })],
  ["language", profile("Choix de langue")],
  ["equation", profile("Valeur calculée")],
]);

export function questionProfile(kind: QuestionKind): QuestionProfile {
  return QUESTION_PROFILES.get(kind) ?? profile("Type importé");
}
