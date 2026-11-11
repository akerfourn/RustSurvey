import type { SurveyQuestion } from "./types.ts";

export const CONSTRAINT_ATTRIBUTES = new Set([
  "min_num_value_n",
  "max_num_value_n",
  "min_answers",
  "max_answers",
  "maximum_chars",
  "em_validation_q",
  "min_num_value",
  "max_num_value",
  "equals_num_value",
]);

const PRESENTATION_ATTRIBUTES = new Set([
  "display_rows",
  "text_input_width",
  "input_size",
  "answer_width",
  "answer_width_bycolumn",
  "question_template",
  "cssclass",
  "page_break",
  "hide_tip",
  "hide_title",
  "hide_help",
  "display_columns",
  "scale_export",
  "statistics_showgraph",
  "statistics_graphtype",
  "public_statistics",
  "chart_type",
]);
const ADMINISTRATION_ATTRIBUTES = new Set(["save_as_default", "clear_default"]);
const TIMER_ATTRIBUTES = new Set([
  "time_limit",
  "time_limit_action",
  "time_limit_countdown_message",
  "time_limit_disable_next",
  "time_limit_disable_prev",
  "time_limit_message",
  "time_limit_message_delay",
  "time_limit_message_style",
  "time_limit_timer_style",
  "time_limit_warning",
  "time_limit_warning_2",
  "time_limit_warning_2_display_time",
  "time_limit_warning_2_message",
  "time_limit_warning_2_style",
  "time_limit_warning_display_time",
  "time_limit_warning_message",
  "time_limit_warning_style",
]);

export function attribute(question: SurveyQuestion, name: string): string {
  return (
    question.attributes.find(
      (item) => item.name === name && (item.language === null || item.language === ""),
    )?.value ?? ""
  );
}

export function isCheckboxMatrix(question: SurveyQuestion): boolean {
  return question.sourceType === ":" && attribute(question, "multiflexible_checkbox") === "1";
}

export function hasActiveTimer(question: SurveyQuestion): boolean {
  return question.attributes.some((item) => item.name === "time_limit" && item.value.trim() !== "");
}

export function isPassiveAttribute(name: string): boolean {
  return (
    PRESENTATION_ATTRIBUTES.has(name) ||
    ADMINISTRATION_ATTRIBUTES.has(name) ||
    TIMER_ATTRIBUTES.has(name)
  );
}

export function displayColumns(question: SurveyQuestion): number {
  const value = attribute(question, "display_columns");
  if (!/^\d+$/.test(value)) return 1;
  return Math.max(1, Math.min(12, Number(value)));
}

export function localizedAttribute(
  question: SurveyQuestion,
  name: string,
  language: string,
  base: string | null,
): string {
  const values = question.attributes.filter((item) => item.name === name);
  return (
    values.find((item) => item.language === language)?.value ??
    values.find((item) => item.language === base)?.value ??
    attribute(question, name)
  );
}

export const INTERACTION_ATTRIBUTES = new Set([
  "hidden",
  "numbers_only",
  "num_value_int_only",
  "use_dropdown",
  "input_boxes",
  "other_comment_mandatory",
  "other_numbers_only",
  "array_filter_style",
  "slider_layout",
  "slider_reset",
  "slider_showminmax",
]);
export const TEXT_ATTRIBUTES = new Set([
  "prefix",
  "suffix",
  "placeholder",
  "dualscale_headerA",
  "dualscale_headerB",
  "other_replace_text",
  "em_validation_q_tip",
  "em_validation_sq_tip",
]);
export const DEFERRED_PRESENTATION_ATTRIBUTES = new Set([
  "question_template",
  "cssclass",
  "page_break",
  "answer_width",
  "answer_width_bycolumn",
  "hide_title",
  "scale_export",
  "statistics_showgraph",
  "statistics_graphtype",
  "public_statistics",
  "chart_type",
]);

export const INACTIVE_ATTRIBUTES = new Set([
  "random_order",
  "reverse",
  "alphasort",
  "exclude_all_others_auto",
  "dropdown_dates",
  "samechoiceheight",
  "samelistheight",
  "show_grand_total",
  "location_mapservice",
  "location_nodefaultfromip",
  "location_postal",
  "location_city",
  "location_state",
  "location_country",
]);
