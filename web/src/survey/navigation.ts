import type { Snapshot, SurveyRuntime } from "./engine.ts";

export interface NavigationSettings {
  format: "group" | "question" | "all";
  showProgress: boolean;
}

export interface SurveyPage {
  id: string;
  groupId: string | null;
  questionIds: string[];
}

export function decodeNavigation(value: unknown): NavigationSettings {
  if (value === null || typeof value !== "object")
    throw new Error("Présentation de campagne invalide.");
  const item = value as Record<string, unknown>;
  if (
    (item.format !== "group" && item.format !== "question" && item.format !== "all") ||
    typeof item.showProgress !== "boolean" ||
    Object.keys(item).some((key) => key !== "format" && key !== "showProgress")
  )
    throw new Error("Présentation de campagne invalide.");
  return { format: item.format, showProgress: item.showProgress };
}

export function createPages(
  runtime: SurveyRuntime,
  snapshot: Snapshot,
  settings: NavigationSettings | null,
): SurveyPage[] {
  const format = settings?.format ?? "group";
  const pages: SurveyPage[] = [];
  const questionsByGroup = new Map<string, string[]>();
  for (const question of runtime.questions.values()) {
    if (!snapshot.visible.has(question.source.id)) continue;
    const ids = questionsByGroup.get(question.source.groupId) ?? [];
    ids.push(question.source.id);
    questionsByGroup.set(question.source.groupId, ids);
  }
  for (const group of runtime.groups) {
    const questionIds = questionsByGroup.get(group.id) ?? [];
    if (format === "question") {
      for (const id of questionIds)
        pages.push({ id: `q:${id}`, groupId: group.id, questionIds: [id] });
    } else if (questionIds.length > 0) {
      pages.push({ id: `g:${group.id}`, groupId: group.id, questionIds });
    }
  }
  if (format === "all")
    return [{ id: "all", groupId: null, questionIds: pages.flatMap((page) => page.questionIds) }];
  if (pages.length === 0 && settings !== null)
    pages.push({ id: "empty", groupId: null, questionIds: [] });
  return pages;
}

export function navigationLabel(settings: NavigationSettings | null): string {
  if (settings?.format === "question") return "Une question par page";
  if (settings?.format === "all") return "Tout sur une page";
  return "Un groupe par page";
}

export function pageCheckpoint(
  page: SurveyPage | undefined,
  language: string,
  settings: NavigationSettings | null,
): { language: string; groupId: string | null; pageId?: string } {
  const checkpoint: { language: string; groupId: string | null; pageId?: string } = {
    language,
    groupId: page?.groupId ?? null,
  };
  if (settings !== null && page !== undefined) checkpoint.pageId = page.id;
  return checkpoint;
}
