import type { RuntimeQuestion } from "./engine.ts";
import type { Answers } from "./types.ts";

export const FILTER_SOURCE_TYPES = new Set([
  "M",
  "P",
  "Q",
  "K",
  "A",
  "B",
  "C",
  "E",
  "F",
  "H",
  "1",
  ":",
  ";",
  "R",
]);

interface SelectionCell {
  key: string;
  row: string;
  checkbox: boolean;
}
export interface SelectionLayout {
  codes: ReadonlySet<string>;
  cells: readonly SelectionCell[];
  ranks: readonly string[];
}
export interface ArrayFilter {
  sourceId: string;
  code: string;
  exclude: boolean;
  selections: SelectionLayout;
}
export type SelectionCache = Map<string, ReadonlySet<string>>;

export function emptySelections(): SelectionLayout {
  return { codes: new Set(), cells: [], ranks: [] };
}

function selectionLayout(question: RuntimeQuestion): SelectionLayout {
  if (question.source.sourceType === "R")
    return {
      codes: new Set(question.choices.map((choice) => choice.code)),
      cells: [],
      ranks: question.cells.map((cell) => cell.key),
    };
  const rows = new Map(
    question.subquestions.filter((row) => row.scale === 0).map((row) => [row.id, row.code]),
  );
  const codes = new Set(rows.values());
  const cells: SelectionCell[] = [];
  for (const cell of question.cells) {
    if (cell.role === "comment") continue;
    let row = rows.get(cell.rowId ?? "");
    if (cell.role === "other") row = "other";
    if (row === undefined) continue;
    codes.add(row);
    cells.push({ key: cell.key, row, checkbox: cell.kind === "checkbox" });
  }
  return { codes, cells, ranks: [] };
}

export function prepareFilters(questions: ReadonlyMap<string, RuntimeQuestion>): void {
  const direct = new Map(
    [...questions.values()].map((question) => [question.source.code, question.filters]),
  );
  const needed = new Set(
    [...direct.values()].flatMap((filters) => filters.map((filter) => filter.code)),
  );
  const layouts = new Map<string, SelectionLayout>();
  for (const question of questions.values())
    if (needed.has(question.source.code) && FILTER_SOURCE_TYPES.has(question.source.sourceType))
      layouts.set(question.source.code, selectionLayout(question));
  for (const question of questions.values()) {
    const pending = [...question.filters];
    const seen = new Set<string>();
    const expanded: ArrayFilter[] = [];
    while (pending.length > 0) {
      const filter = pending.pop()!;
      const key = `${filter.code}:${String(filter.exclude)}`;
      if (seen.has(key)) continue;
      seen.add(key);
      const selections = layouts.get(filter.code);
      if (selections === undefined) continue;
      expanded.push({ ...filter, selections });
      pending.push(...(direct.get(filter.code) ?? []));
    }
    question.filters = expanded;
  }
}

function selectedCodes(layout: SelectionLayout, answers: Answers): ReadonlySet<string> {
  const selected = new Set<string>();
  for (const cell of layout.cells) {
    const value = answers.get(cell.key) ?? "";
    if (value !== "" && (!cell.checkbox || value === "1")) selected.add(cell.row);
  }
  for (const key of layout.ranks) {
    const value = answers.get(key);
    if (value !== undefined && layout.codes.has(value)) selected.add(value);
  }
  return selected;
}

export function filterAllows(
  filters: readonly ArrayFilter[],
  row: string,
  answers: Answers,
  cache: SelectionCache,
): boolean {
  for (const filter of filters) {
    if (!filter.selections.codes.has(row)) continue;
    let selected = cache.get(filter.code);
    if (selected === undefined) {
      selected = selectedCodes(filter.selections, answers);
      cache.set(filter.code, selected);
    }
    if (selected.has(row) === filter.exclude) return false;
  }
  return true;
}

export function applyChoiceFilters(
  question: RuntimeQuestion,
  answers: Map<string, string>,
  cache: SelectionCache,
): ReadonlySet<string> | null {
  const type = question.source.sourceType;
  if (question.filters.length === 0 || !["L", "R"].includes(type)) return null;
  const codes = new Set(question.choices.map((choice) => choice.code));
  const allowed = new Set(
    [...codes].filter((code) => filterAllows(question.filters, code, answers, cache)),
  );
  if (type === "L") {
    if (question.source.other) allowed.add("-oth-");
    const value = answers.get(question.source.code) ?? "";
    if ((codes.has(value) || (question.source.other && value === "-oth-")) && !allowed.has(value))
      answers.delete(question.source.code);
    return allowed;
  }
  normalizeRanking(question, answers, codes, allowed);
  return allowed;
}

function normalizeRanking(
  question: RuntimeQuestion,
  answers: Map<string, string>,
  codes: ReadonlySet<string>,
  allowed: ReadonlySet<string>,
): void {
  const retained: string[] = [];
  const unique = new Set<string>();
  let removed = false;
  for (const cell of question.cells) {
    const value = answers.get(cell.key) ?? "";
    if (value === "") continue;
    if (!codes.has(value)) throw new Error("Choix de réponse inconnu.");
    if (unique.has(value))
      throw new Error("Chaque choix doit occuper une seule position du classement.");
    unique.add(value);
    if (allowed.has(value)) retained.push(value);
    else removed = true;
  }
  if (removed)
    for (const [index, cell] of question.cells.entries()) {
      const value = retained[index];
      if (value === undefined) answers.delete(cell.key);
      else answers.set(cell.key, value);
    }
  if (
    !removed &&
    question.cells.slice(allowed.size).some((cell) => (answers.get(cell.key) ?? "") !== "")
  )
    throw new Error("Le classement ne doit pas contenir de position vide entre deux choix.");
}
