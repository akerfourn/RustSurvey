import { evaluateSnapshot, attribute, type Snapshot, type SurveyRuntime } from "./engine.ts";
import { createPages, type NavigationSettings, type SurveyPage } from "./navigation.ts";
import type { Answers } from "./types.ts";

export interface PageEntry {
  snapshot: Snapshot;
  page: SurveyPage | undefined;
}

export function enterPage(
  runtime: SurveyRuntime,
  answers: Answers,
  settings: NavigationSettings | null,
  language: string,
  previousId?: string,
  requestedId?: string,
  submit = false,
): PageEntry {
  const budget = { remaining: 1_000_000 };
  const working = new Map(answers);
  const scope = new Set<string>();
  const evaluate = (initialize?: ReadonlySet<string>): Snapshot =>
    evaluateSnapshot(runtime, working, { language, budget, initialize });
  const resolve = (snapshot: Snapshot, id?: string): SurveyPage | undefined => {
    const pages = createPages(runtime, snapshot, settings);
    return pages.find((page) => page.id === id) ?? pages[0];
  };
  function probe(): Snapshot {
    const snapshot = evaluate(scope);
    for (const key of snapshot.initialized) working.set(key, snapshot.answers.get(key)!);
    return snapshot;
  }
  let snapshot = evaluate();
  if (previousId !== undefined) {
    const previous = resolve(snapshot, previousId);
    addPageQuestions(runtime, scope, settings, previous);
    addHiddenPrefix(runtime, scope, settings, previous);
    snapshot = probe();
  }
  let page = resolve(snapshot, requestedId);
  addHiddenPrefix(runtime, scope, settings, page);
  snapshot = probe();
  page = resolve(snapshot, requestedId);
  addPageQuestions(runtime, scope, settings, page);
  if (submit) {
    for (const id of runtime.questions.keys()) scope.add(id);
  }
  snapshot = evaluate(scope);
  return { snapshot, page: resolve(snapshot, page?.id) };
}

function addPageQuestions(
  runtime: SurveyRuntime,
  scope: Set<string>,
  settings: NavigationSettings | null,
  page: SurveyPage | undefined,
): void {
  for (const [id, question] of runtime.questions) {
    if (settings?.format === "all") scope.add(id);
    else if (settings?.format === "question") {
      if (page?.questionIds.includes(id) === true) scope.add(id);
    } else if (question.source.groupId === page?.groupId) scope.add(id);
  }
}

function addHiddenPrefix(
  runtime: SurveyRuntime,
  scope: Set<string>,
  settings: NavigationSettings | null,
  page: SurveyPage | undefined,
): void {
  for (const group of runtime.groups) {
    for (const [id, question] of runtime.questions) {
      if (question.source.groupId !== group.id) continue;
      if (attribute(question.source, "hidden") === "1") scope.add(id);
      if (settings?.format === "question" && page?.questionIds.includes(id) === true) return;
    }
    if ((settings === null || settings.format === "group") && group.id === page?.groupId) return;
  }
}
