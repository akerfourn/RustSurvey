import { RuntimeInspector } from "../authoring/RuntimeInspector.tsx";
import { useMemo, useState } from "preact/hooks";
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
import { StudyPage } from "./StudyPage.tsx";
import { ViewerActions, ViewerHeader } from "./ViewerControls.tsx";
import { SurveyMessage } from "./SurveyMessage.tsx";
import type { ViewerPersistence } from "../collection/types.ts";
import { sourceBehaviorIssues } from "../survey/source-behavior.ts";

interface StudyViewerProps {
  study: SurveyStudy;
  inspection?: Inspection;
  report?: InspectionReport | undefined;
  debug?: boolean;
  persistence?: ViewerPersistence;
}

function focusQuestion(id: string | undefined): void {
  if (id === undefined) return;
  const element = document.getElementById(`question-${id}`);
  element?.focus({ preventScroll: true });
  element?.scrollIntoView({ block: "center" });
}

export function StudyViewer({
  study,
  inspection,
  report: sourceReport,
  debug = false,
  persistence,
}: StudyViewerProps) {
  const report = sourceReport ?? inspection?.report ?? { errors: 0, sections: [] };
  const runtime = useMemo(() => createRuntime(study), [study]);
  let mediaBaseUrl: string | null = null;
  if (study.surveyId !== null) {
    const prefix = debug ? "/api/admin/studies" : "/api/studies";
    mediaBaseUrl = `${prefix}/${encodeURIComponent(study.surveyId)}/media`;
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

  function changeAnswers(changes: readonly (readonly [string, string])[]): void {
    const updated = changeExclusiveAnswers(runtime, answers, changes);
    updateAnswers(updated, currentPage?.id);
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
    const updated = new Map(answers);
    for (const question of runtime.questions.values()) {
      if (question.source.sourceType === "I") updated.set(question.source.code, value);
    }
    updateAnswers(updated, currentPage?.id, false, value);
    setLanguage(value);
  }
  function handleStart(): void {
    setStarted(true);
  }
  function handleEdit(): void {
    setComplete(false);
  }
  function handlePrevious(): void {
    if (persistence?.disabled === true) return;
    if (position <= 0) setStarted(false);
    else if (navigation !== null && persistence !== undefined && !readOnly)
      savePage(pages[position - 1], false);
    else movePage(pages[position - 1]);
    setAttempted(false);
  }
  function savePage(page: SurveyPage | undefined, submit: boolean): void {
    const entry = updateAnswers(answers, page?.id, submit);
    persistence?.onSave(
      { answers: entry.snapshot.answers, ...pageCheckpoint(entry.page, language, navigation) },
      submit,
    );
  }
  function movePage(page: SurveyPage | undefined): void {
    const entry = updateAnswers(answers, page?.id);
    setPageId(entry.page?.id);
  }
  function handleNext(): void {
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
        savePage(pages[position + 1], false);
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
      } else savePage(currentPage, true);
    }
  }
  function handleSaveDraft(): void {
    savePage(currentPage, false);
  }
  function handleDownload(): void {
    downloadAnswers(study, language, snapshot, persistence?.submitted ?? complete);
  }
  function handleReset(): void {
    const entry = updateAnswers(new Map(), undefined);
    setPageId(entry.page?.id);
    setStarted(false);
    setAttempted(false);
    setComplete(false);
  }
  const issueItems = issues.map((issue, index) => <li key={`${index}-${issue}`}>{issue}</li>);
  let issueContent = null;
  if (issues.length > 0)
    issueContent = (
      <details className="notice warning">
        <summary>{issues.length} limite(s) du moteur d’aperçu</summary>
        <ul>{issueItems}</ul>
        <p>Les questions concernées restent consultables. La validation finale est désactivée.</p>
      </details>
    );
  let noticeContent = null;
  if (notices.length > 0)
    noticeContent = (
      <details className="notice">
        <summary>{notices.length} différence(s) de présentation</summary>
        <ul>
          {notices.map((notice) => (
            <li key={notice}>{notice}</li>
          ))}
        </ul>
      </details>
    );
  let readOnlyContent = null;
  if (report.errors > 0 || runtime.issues.length > 0)
    readOnlyContent = (
      <p className="notice error">
        La structure contient des erreurs ou dépasse les limites de l’aperçu. La saisie est
        désactivée.
      </p>
    );
  let nextLabel = "Groupe suivant";
  if (navigation?.format === "question") nextLabel = "Question suivante";
  if (position === pages.length - 1) nextLabel = "Valider la saisie locale";
  if (position === pages.length - 1 && persistence !== undefined)
    nextLabel = "Envoyer mes réponses";
  if (position === pages.length - 1 && persistence?.readOnly === true)
    nextLabel = "Terminer la consultation";
  if (
    !readOnly &&
    attempted &&
    currentValidation.errors.size === 0 &&
    currentValidation.warnings.size > 0
  )
    nextLabel = "Continuer sans les réponses recommandées";
  let pageContent = (
    <p className="notice">Aucune question pertinente à afficher pour les réponses actuelles.</p>
  );
  let messageMode: "local" | "submitted" | "closed" = "local";
  if (persistence?.submitted === true) messageMode = "submitted";
  else if (persistence?.readOnly === true) messageMode = "closed";
  let nextDisabled = invalidStructure || persistence?.disabled === true;
  if (persistence?.readOnly !== true && position === pages.length - 1 && cannotComplete)
    nextDisabled = true;
  if (hasWelcome && !started)
    pageContent = <SurveyMessage message={welcome} completed={false} onContinue={handleStart} />;
  else if (complete)
    pageContent = (
      <SurveyMessage message={ending} completed={true} mode={messageMode} onContinue={handleEdit} />
    );
  else if (currentPage !== undefined)
    pageContent = (
      <StudyPage
        runtime={runtime}
        mediaBaseUrl={mediaBaseUrl}
        snapshot={snapshot}
        page={currentPage}
        pageCount={pages.length}
        format={navigation?.format ?? "group"}
        showProgress={navigation?.showProgress ?? true}
        language={language}
        position={position}
        validation={currentValidation}
        attempted={attempted}
        readOnly={readOnly}
        onAnswer={handleAnswer}
        onClear={handleClear}
        onLanguage={handleLanguageCode}
        previousDisabled={(position <= 0 && !hasWelcome) || persistence?.disabled === true}
        nextDisabled={nextDisabled}
        nextLabel={nextLabel}
        onPrevious={handlePrevious}
        onNext={handleNext}
      />
    );
  let inspector = null;
  if (debug) inspector = <RuntimeInspector runtime={runtime} snapshot={snapshot} />;
  let actions = <ViewerActions onDownload={handleDownload} onReset={handleReset} />;
  if (persistence !== undefined)
    actions = (
      <div className="local-actions">
        <button type="button" onClick={handleSaveDraft} disabled={readOnly}>
          Enregistrer pour reprendre plus tard
        </button>
        <button type="button" className="secondary" onClick={handleDownload}>
          Télécharger une copie JSON
        </button>
      </div>
    );
  return (
    <section className="viewer panel" aria-labelledby="viewer-title">
      {inspector}
      <ViewerHeader
        language={language}
        languages={study.languages}
        onLanguage={handleLanguageCode}
        durable={persistence !== undefined}
        disabled={readOnly}
      />
      {readOnlyContent}
      {issueContent}
      {noticeContent}
      {pageContent}
      {actions}
    </section>
  );
}
