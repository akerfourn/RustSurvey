import { useMemo, useState } from "preact/hooks";
import { evaluateSnapshot, type Snapshot, type SurveyRuntime } from "./engine.ts";
import { enterPage, type PageEntry } from "./initialization.ts";
import { createPages, type NavigationSettings } from "./navigation.ts";
import type { Answers } from "./types.ts";

interface Options {
  runtime: SurveyRuntime;
  initialAnswers: Answers | undefined;
  navigation: NavigationSettings | null;
  language: string;
  pageId: string | undefined;
  initialize: boolean;
}

export interface ViewerAnswers {
  answers: Answers;
  snapshot: Snapshot;
  update: (
    answers: Answers,
    requestedId: string | undefined,
    submit?: boolean,
    selectedLanguage?: string,
  ) => PageEntry;
}

export function useViewerAnswers({
  runtime,
  initialAnswers,
  navigation,
  language,
  pageId,
  initialize,
}: Options): ViewerAnswers {
  function prepare(
    answers: Answers,
    requestedId: string | undefined,
    submit: boolean,
    previousId?: string,
  ): PageEntry {
    if (initialize)
      return enterPage(runtime, answers, navigation, language, previousId, requestedId, submit);
    const snapshot = evaluateSnapshot(runtime, answers);
    const pages = createPages(runtime, snapshot, navigation);
    return { snapshot, page: pages.find((page) => page.id === requestedId) ?? pages[0] };
  }
  const [state, setState] = useState(() => {
    const entry = prepare(new Map(initialAnswers), pageId, false);
    return { answers: entry.snapshot.answers, issues: entry.snapshot.issues };
  });
  const snapshot = useMemo(() => {
    let result: Snapshot;
    if (initialize) result = evaluateSnapshot(runtime, state.answers, { language });
    else result = evaluateSnapshot(runtime, state.answers);
    result.issues = [...new Set([...result.issues, ...state.issues])];
    return result;
  }, [runtime, state, language, initialize]);

  function update(
    answers: Answers,
    requestedId: string | undefined,
    submit = false,
    selectedLanguage = language,
  ): PageEntry {
    let entry: PageEntry;
    if (initialize)
      entry = enterPage(
        runtime,
        answers,
        navigation,
        selectedLanguage,
        pageId,
        requestedId,
        submit,
      );
    else entry = prepare(answers, requestedId, submit, pageId);
    setState({ answers: entry.snapshot.answers, issues: entry.snapshot.issues });
    return entry;
  }
  return { answers: state.answers, snapshot, update };
}
