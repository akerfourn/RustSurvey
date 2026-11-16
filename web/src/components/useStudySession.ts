import { publicMediaPath } from "../authoring/publication-links.ts";
import { useMemo, useRef, useState } from "preact/hooks";
import {
  acknowledgeProgress,
  beginSave,
  editProgress,
  hasUnsavedChanges,
  initialProgress,
  type ProgressState,
} from "../collection/progress.ts";
import { useAutosave } from "../collection/useAutosave.ts";
import { useUnsavedChanges } from "../navigation/useUnsavedChanges.ts";
import { createRuntime, runtimeIssues } from "../survey/engine.ts";
import { useViewerAnswers } from "../survey/use-viewer-answers.ts";
import {
  studyTextIssues,
  presentationNotices,
  compileRuntimeTexts,
  textExecutionIssues,
} from "../survey/capabilities.ts";
import { downloadAnswers } from "../survey/download.ts";
import { localized, pipeText, plainText } from "../survey/text.ts";
import type { Inspection, InspectionReport, SurveyStudy } from "../survey/types.ts";
import { validate } from "../survey/validation.ts";
import { changeExclusiveAnswers } from "../survey/exclusive.ts";
import { createPages, pageCheckpoint, type SurveyPage } from "../survey/navigation.ts";
import type { ViewerPersistence } from "../collection/types.ts";
import { sourceBehaviorIssues } from "../survey/source-behavior.ts";

export interface StudyViewerProps {
  study: SurveyStudy;
  inspection?: Inspection;
  report?: InspectionReport | undefined;
  debug?: boolean;
  persistence?: ViewerPersistence;
  publishedRevision?: number;
}

function focusQuestion(id: string | undefined): void {
  if (id === undefined) return;
  const element = document.getElementById(`question-${id}`);
  element?.focus({ preventScroll: true });
  element?.scrollIntoView({ block: "center" });
}

export function useStudySession({
  study,
  inspection,
  report: sourceReport,
  debug = false,
  persistence,
  publishedRevision,
}: StudyViewerProps) {
  const report = sourceReport ?? inspection?.report ?? { errors: 0, sections: [] };
  const runtime = useMemo(() => createRuntime(study), [study]);
  let mediaBaseUrl: string | null = null;
  if (study.surveyId !== null) {
    const prefix = debug ? "/api/admin/studies" : "/api/studies";
    mediaBaseUrl = `${prefix}/${encodeURIComponent(study.surveyId)}/media`;
    if (!debug && publishedRevision !== undefined) {
      mediaBaseUrl = publicMediaPath(study.surveyId, publishedRevision);
    }
  }
  const textProblems = useMemo(() => studyTextIssues(runtime), [runtime]);
  const notices = useMemo(() => presentationNotices(runtime), [runtime]);
  const [language, setLanguage] = useState(
    persistence?.language ?? study.baseLanguage ?? study.languages[0] ?? "fr",
  );
  const navigation = persistence?.navigation ?? null;
  const [pageId, setPageId] = useState<string | undefined>(() => {
    if (persistence?.pageId !== undefined) return persistence.pageId;
    if (persistence?.groupId !== null && persistence?.groupId !== undefined)
      return `g:${persistence.groupId}`;
    return undefined;
  });
  const hasDefaults = [...runtime.questions.values()].some(
    (question) => (question.defaults?.fields.size ?? 0) > 0,
  );
  const initialize =
    hasDefaults &&
    (persistence === undefined ||
      (persistence.profile === "native-defaults-v1" &&
        !persistence.readOnly &&
        !persistence.submitted));
  const {
    answers,
    snapshot,
    update: updateAnswers,
    replace: replaceAnswers,
  } = useViewerAnswers({
    runtime,
    initialAnswers: persistence?.answers,
    navigation,
    language,
    pageId,
    initialize,
  });
  const [started, setStarted] = useState(persistence?.hasProgress ?? false);
  const [attempted, setAttempted] = useState(false);
  const [complete, setComplete] = useState(persistence?.submitted ?? false);
  const compiledTexts = useMemo(() => compileRuntimeTexts(runtime, language), [runtime, language]);
  const pages = useMemo(
    () => createPages(runtime, snapshot, navigation),
    [runtime, snapshot, navigation],
  );
  const currentPage = pages.find((page) => page.id === pageId) ?? pages[0];
  const position = pages.findIndex((page) => page.id === currentPage?.id);
  const [progress, setProgress] = useState(() =>
    initialProgress({
      answers: snapshot.answers,
      ...pageCheckpoint(currentPage, language, navigation),
    }),
  );
  const latestProgress = useRef(progress);
  const inFlight = useRef(false);
  const dirty = hasUnsavedChanges(progress);
  useUnsavedChanges(
    persistence !== undefined,
    () =>
      hasUnsavedChanges(latestProgress.current) ||
      inFlight.current ||
      persistence?.uncertain === true,
  );
  let validationGroup: string | undefined;
  let validationQuestions: Set<string> | undefined;
  if (navigation === null) validationGroup = currentPage?.groupId ?? undefined;
  else validationQuestions = new Set(currentPage?.questionIds);
  const currentValidation = validate(runtime, snapshot, validationGroup, validationQuestions);
  const messages = localized(study.messages, language, study.baseLanguage);
  const welcome = pipeText(plainText(messages?.welcome), snapshot.answers, runtime.fields);
  const ending = pipeText(plainText(messages?.end), snapshot.answers, runtime.fields);
  const hasWelcome = study.showWelcome === true && welcome.text !== "";
  const issues = [
    ...runtimeIssues(runtime, snapshot),
    ...textProblems,
    ...textExecutionIssues(compiledTexts, snapshot),
    ...welcome.issues,
    ...ending.issues,
  ];
  if (study.showWelcome === null) {
    issues.push(
      "L'affichage de l'accueil dépend d'un réglage source hérité non fourni. Cet aperçu ne peut pas certifier le parcours complet.",
    );
  }
  issues.push(...sourceBehaviorIssues(report));
  const invalidStructure = report.errors > 0 || runtime.issues.length > 0;
  const readOnly =
    invalidStructure ||
    persistence?.readOnly === true ||
    persistence?.submitted === true ||
    persistence?.disabled === true;
  const cannotComplete = readOnly || issues.length > 0;
  const navigationDisabled =
    invalidStructure || persistence?.disabled === true || persistence?.saving === true;
  useAutosave(
    {
      editVersion: progress.editVersion,
      attemptedVersion: progress.attemptedVersion,
      dirty,
      enabled: persistence !== undefined && !readOnly && !persistence.saving && !complete,
    },
    () => {
      void persist(currentPage, false, true);
    },
  );

  function updateProgress(value: ProgressState): void {
    latestProgress.current = value;
    setProgress(value);
  }
  function changeAnswers(changes: readonly (readonly [string, string])[]): void {
    if (readOnly) return;
    const updated = changeExclusiveAnswers(runtime, latestProgress.current.answers, changes);
    const entry = updateAnswers(updated, currentPage?.id);
    updateProgress(
      editProgress(latestProgress.current, {
        answers: entry.snapshot.answers,
        ...pageCheckpoint(entry.page, language, navigation),
      }),
    );
    setStarted(true);
    setComplete(false);
  }
  function handleAnswer(field: string, value: string): void {
    changeAnswers([[field, value]]);
  }
  function handleClear(fields: readonly string[]): void {
    changeAnswers(fields.map((field) => [field, ""]));
  }
  function handleLanguageCode(value: string): void {
    if (readOnly) return;
    const updated = new Map(latestProgress.current.answers);
    for (const question of runtime.questions.values()) {
      if (question.source.sourceType === "I") updated.set(question.source.code, value);
    }
    const entry = updateAnswers(updated, currentPage?.id, false, value);
    updateProgress(
      editProgress(latestProgress.current, {
        answers: entry.snapshot.answers,
        ...pageCheckpoint(entry.page, value, navigation),
      }),
    );
    setLanguage(value);
  }
  function handleStart(): void {
    setStarted(true);
  }
  function handleEdit(): void {
    setComplete(false);
  }
  async function handlePrevious(): Promise<void> {
    if (navigationDisabled) return;
    if (persistence !== undefined && !readOnly) {
      const target = pages[Math.max(0, position - 1)];
      if (!(await persist(target, false))) return;
    } else if (position > 0) movePage(pages[position - 1]);
    if (position <= 0) setStarted(false);
    setAttempted(false);
  }
  async function persist(
    page: SurveyPage | undefined,
    submit: boolean,
    background = false,
  ): Promise<boolean> {
    if (
      persistence === undefined ||
      inFlight.current ||
      persistence.saving ||
      persistence.blocked ||
      persistence.readOnly ||
      persistence.submitted
    )
      return false;
    const current = latestProgress.current;
    const entry = updateAnswers(current.answers, page?.id, submit, current.language);
    const sent = beginSave(
      editProgress(current, {
        answers: entry.snapshot.answers,
        ...pageCheckpoint(entry.page, current.language, navigation),
      }),
    );
    updateProgress(sent);
    inFlight.current = true;
    try {
      const saved = await persistence.onSave(sent, submit, background);
      if (saved === null) return false;
      const latest = latestProgress.current;
      updateProgress(acknowledgeProgress(latest, sent.editVersion, saved));
      if (latest.editVersion === sent.editVersion) {
        replaceAnswers(saved.answers);
        setLanguage(saved.language);
        let target = saved.pageId;
        if (target === undefined && saved.groupId !== null) target = `g:${saved.groupId}`;
        setPageId(target);
      }
      setStarted(true);
      setComplete(saved.acknowledgement.status === "submitted");
      return true;
    } finally {
      inFlight.current = false;
    }
  }
  function movePage(page: SurveyPage | undefined): void {
    const entry = updateAnswers(answers, page?.id);
    setPageId(entry.page?.id);
    updateProgress({
      ...latestProgress.current,
      answers: entry.snapshot.answers,
      ...pageCheckpoint(entry.page, language, navigation),
    });
  }
  async function handleNext(): Promise<void> {
    if (navigationDisabled) return;
    if (persistence?.readOnly === true) {
      if (position < pages.length - 1) setPageId(pages[position + 1]?.id);
      else setComplete(true);
      setAttempted(false);
      return;
    }
    setStarted(true);
    setAttempted(true);
    if (currentValidation.errors.size > 0) {
      focusQuestion(currentValidation.errors.keys().next().value);
      return;
    }
    if (currentValidation.warnings.size > 0 && !attempted) return;
    if (position < pages.length - 1) {
      if (persistence !== undefined) {
        if (await persist(pages[position + 1], false)) setAttempted(false);
        return;
      }
      movePage(pages[position + 1]);
      setAttempted(false);
      return;
    }
    const all = validate(runtime, snapshot);
    if (all.errors.size > 0) {
      const firstPage = pages.find((page) => page.questionIds.some((id) => all.errors.has(id)));
      setPageId(firstPage?.id);
      return;
    }
    if (!cannotComplete) {
      if (persistence === undefined) {
        const entry = updateAnswers(answers, currentPage?.id, true);
        if (
          validate(runtime, entry.snapshot).errors.size === 0 &&
          entry.snapshot.issues.length === 0
        )
          setComplete(true);
      } else await persist(currentPage, true);
    }
  }
  function handleSaveDraft(): void {
    void persist(currentPage, false);
  }
  function handleDownload(): void {
    downloadAnswers(study, language, snapshot, persistence?.submitted ?? complete);
  }
  function handleReset(): void {
    if (persistence !== undefined) return;
    const entry = updateAnswers(new Map(), undefined);
    updateProgress(
      editProgress(latestProgress.current, {
        answers: entry.snapshot.answers,
        ...pageCheckpoint(entry.page, language, navigation),
      }),
    );
    setPageId(entry.page?.id);
    setStarted(false);
    setAttempted(false);
    setComplete(false);
  }
  return {
    runtime,
    mediaBaseUrl,
    snapshot,
    language,
    currentPage,
    pages,
    position,
    currentValidation,
    welcome,
    ending,
    hasWelcome,
    issues,
    notices,
    invalidStructure,
    readOnly,
    cannotComplete,
    started,
    attempted,
    complete,
    handleAnswer,
    handleClear,
    handleLanguageCode,
    handleStart,
    handleEdit,
    handlePrevious,
    handleNext,
    handleSaveDraft,
    handleDownload,
    handleReset,
    navigation,
    dirty,
    navigationDisabled,
  };
}
