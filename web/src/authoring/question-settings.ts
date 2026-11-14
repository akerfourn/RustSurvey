import type { Question, QuestionKind } from "./model.ts";
import { questionProfile } from "./question-profile.ts";

export interface QuestionSetting {
  name: string;
  label: string;
  kind: "text" | "boolean" | "expression" | "number";
  hint?: string;
}

export function settingValue(question: Question, name: string): string {
  return (
    question.attributes.find(
      (item) => item.name === name && (item.language === null || item.language === ""),
    )?.value ?? ""
  );
}

export function setSetting(question: Question, name: string, value: string): Question {
  const attributes = question.attributes.filter(
    (item) => item.name !== name || (item.language !== null && item.language !== ""),
  );
  if (value !== "") attributes.push({ name, value, language: null });
  return { ...question, attributes };
}

export function questionSettings(kind: QuestionKind): QuestionSetting[] {
  const settings: QuestionSetting[] = [
    {
      name: "hidden",
      label: "Masquer la question (la valeur calculée reste disponible)",
      kind: "boolean",
    },
  ];
  if (kind === "information") return settings;
  if (kind === "equation") {
    settings.push({
      name: "equation",
      label: "Expression calculée",
      kind: "expression",
      hint: "Exemple : sum(PRIX, TAXE). La valeur est recalculée à partir des réponses actives.",
    });
    return settings;
  }
  const profile = questionProfile(kind);
  const numeric = ["number", "multiple_numbers", "number_matrix"].includes(kind);
  const textual = ["short_text", "long_text", "huge_text", "multiple_text", "text_matrix"].includes(
    kind,
  );
  settings.push(
    {
      name: "em_validation_q",
      label: "Règle de validation de la question",
      kind: "expression",
      hint: "Exemple : AGE >= 18. Cette règle peut lire la réponse de la question elle-même.",
    },
    { name: "em_validation_q_tip", label: "Message si la validation échoue", kind: "text" },
  );
  if (numeric)
    settings.push(
      { name: "min_num_value_n", label: "Valeur minimale par cellule", kind: "expression" },
      { name: "max_num_value_n", label: "Valeur maximale par cellule", kind: "expression" },
      { name: "num_value_int_only", label: "Nombres entiers uniquement", kind: "boolean" },
      { name: "prefix", label: "Préfixe", kind: "text" },
      { name: "suffix", label: "Suffixe", kind: "text" },
    );
  if (numeric && profile.rows)
    settings.push(
      { name: "min_num_value", label: "Somme minimale", kind: "expression" },
      { name: "max_num_value", label: "Somme maximale", kind: "expression" },
      { name: "equals_num_value", label: "Somme attendue", kind: "expression" },
    );
  if (textual)
    settings.push(
      {
        name: "maximum_chars",
        label: "Nombre maximal de caractères par cellule",
        kind: "expression",
      },
      { name: "placeholder", label: "Texte indicatif", kind: "text" },
    );
  if (["short_text", "multiple_text"].includes(kind))
    settings.push({ name: "numbers_only", label: "Saisie numérique", kind: "boolean" });
  if (["long_text", "huge_text"].includes(kind))
    settings.push({ name: "display_rows", label: "Hauteur du champ (lignes)", kind: "number" });
  if (profile.rows || kind === "ranking")
    settings.push(
      { name: "min_answers", label: "Nombre minimal de réponses", kind: "expression" },
      { name: "max_answers", label: "Nombre maximal de réponses", kind: "expression" },
    );
  if (profile.rows || ["single_choice", "ranking"].includes(kind)) {
    let filterHint =
      "Codes de questions séparés par ; : choix, champs ou tableaux multiples, classement. Les codes des lignes ou choix doivent correspondre. Le code other désigne Autre.";
    if (kind === "array_column")
      filterHint += " Le filtre de ce tableau n’est pas exportable vers LimeSurvey 6.5.14.";
    settings.push(
      {
        name: "array_filter",
        label: "Filtre : questions source à inclure",
        kind: "text",
        hint: filterHint,
      },
      {
        name: "array_filter_exclude",
        label: "Filtre : questions source à exclure",
        kind: "text",
      },
      {
        name: "array_filter_style",
        label: "Afficher les lignes filtrées désactivées",
        kind: "boolean",
      },
    );
  }
  if (profile.other)
    settings.push(
      { name: "other_replace_text", label: "Libellé de la réponse « autre »", kind: "text" },
      { name: "other_numbers_only", label: "Réponse « autre » numérique", kind: "boolean" },
    );
  if (
    [
      "multiple_choice",
      "multiple_choice_comment",
      "multiple_numbers",
      "multiple_text",
      "array_five",
      "array_ten",
      "array_yes_no",
      "array_increase",
      "array_choice",
    ].includes(kind)
  )
    settings.push({
      name: "exclude_all_others",
      label: "Codes des lignes exclusives",
      kind: "text",
      hint: "Codes séparés par ;. Renseigner une ligne exclusive efface et désactive les autres lignes. Le code other désigne l’option Autre si elle est activée.",
    });
  if (["multiple_choice", "multiple_choice_comment"].includes(kind))
    settings.push({
      name: "exclude_all_others_auto",
      label: "Sélectionner l’option exclusive quand toutes les autres sont remplies",
      kind: "boolean",
      hint: "Ne s’applique qu’avec un seul code exclusif, sans point-virgule.",
    });
  if (kind === "single_choice" || kind === "dropdown")
    settings.push({
      name: "other_comment_mandatory",
      label: "Précision obligatoire pour « autre »",
      kind: "boolean",
    });
  if (kind === "multiple_numbers")
    settings.push(
      { name: "slider_layout", label: "Utiliser des curseurs", kind: "boolean" },
      { name: "slider_min", label: "Minimum des curseurs", kind: "number" },
      { name: "slider_max", label: "Maximum des curseurs", kind: "number" },
      { name: "slider_accuracy", label: "Pas des curseurs", kind: "number" },
      { name: "slider_showminmax", label: "Afficher les bornes", kind: "boolean" },
      { name: "slider_reset", label: "Permettre de vider un curseur", kind: "boolean" },
    );
  if (kind === "number_matrix")
    settings.push(
      { name: "multiflexible_checkbox", label: "Utiliser des cases à cocher", kind: "boolean" },
      { name: "input_boxes", label: "Saisie libre au lieu des listes de nombres", kind: "boolean" },
      { name: "multiflexible_min", label: "Minimum de la matrice", kind: "number" },
      { name: "multiflexible_max", label: "Maximum de la matrice", kind: "number" },
      { name: "multiflexible_step", label: "Pas des listes de nombres", kind: "number" },
    );
  if (kind === "dual_scale")
    settings.push(
      { name: "dualscale_headerA", label: "Titre de la première échelle", kind: "text" },
      { name: "dualscale_headerB", label: "Titre de la seconde échelle", kind: "text" },
    );
  if (kind === "single_choice" || kind === "dropdown")
    settings.push({ name: "use_dropdown", label: "Rendu en liste déroulante", kind: "boolean" });
  return settings;
}
