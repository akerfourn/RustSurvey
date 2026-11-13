import { RuntimeInspector } from "../authoring/RuntimeInspector.tsx";
import { useMemo, useState } from "preact/hooks";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "../survey/engine.ts";
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
import { StudyPage } from "./StudyPage.tsx";
import { ViewerActions, ViewerHeader } from "./ViewerControls.tsx";
import { SurveyMessage } from "./SurveyMessage.tsx";
import type { ViewerPersistence } from "../collection/types.ts";

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
  const [answers, setAnswers] = useState(() => new Map(persistence?.answers));
  const [language, setLanguage] = useState(
    persistence?.language ?? study.baseLanguage ?? study.languages[0] ?? "fr",
  );
  const [groupId, setGroupId] = useState<string | undefined>(
    persistence?.groupId ?? study.groups[0]?.id,
  );
  const [started, setStarted] = useState(persistence?.hasProgress ?? false);
  const [attempted, setAttempted] = useState(false);
  const [complete, setComplete] = useState(persistence?.submitted ?? false);
  const snapshot = useMemo(() => evaluateSnapshot(runtime, answers), [runtime, answers]);
  const compiledTexts = useMemo(() => compileRuntimeTexts(runtime, language), [runtime, language]);
  const currentGroup = snapshot.groups.find((group) => group.id === groupId) ?? snapshot.groups[0];
  const position = snapshot.groups.findIndex((group) => group.id === currentGroup?.id);
  const currentValidation = validate(runtime, snapshot, currentGroup?.id);
  const messages = localized(study.messages, language, study.baseLanguage);
  const welcome = pipeText(plainText(messages?.welcome), snapshot.answers, runtime.fields);
  const ending = pipeText(plainText(messages?.end), snapshot.answers, runtime.fields);
  const hasWelcome = study.showWelcome && welcome.text !== "";
  const issues = [
    ...runtimeIssues(runtime, snapshot),
    ...textProblems,
    ...textExecutionIssues(compiledTexts, snapshot),
    ...welcome.issues,
    ...ending.issues,
  ];
  for (const section of report.sections) {
    if (
      ["conditions", "defaultvalues", "quota", "quota_members", "quota_languagesettings"].includes(
        section.name,
      ) &&
      section.rows > 0
    ) {
      issues.push(`Section ${section.name} : comportement non pris en charge par cet aperçu.`);
    }
  }
  const invalidStructure = report.errors > 0 || runtime.issues.length > 0;
  const readOnly =
    invalidStructure ||
    persistence?.readOnly === true ||
    persistence?.submitted === true ||
    persistence?.disabled === true;
  const cannotComplete = readOnly || issues.length > 0;

  function changeAnswers(changes: readonly (readonly [string, string])[]): void {
    setAnswers((previous) => {
      const updated = new Map(previous);
      for (const [field, value] of changes) updated.set(field, value);
      return evaluateSnapshot(runtime, updated).answers;
    });
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
    setAnswers((previous) => {
      const updated = new Map(previous);
      for (const question of runtime.questions.values()) {
        if (question.source.sourceType === "I") updated.set(question.source.code, value);
      }
      return evaluateSnapshot(runtime, updated).answers;
    });
    setLanguage(value);
  }
  function handleStart(): void {
    setStarted(true);
  }
  function handleEdit(): void {
    setComplete(false);
  }
  function handlePrevious(): void {
    if (position <= 0) setStarted(false);
    else setGroupId(snapshot.groups[position - 1]?.id);
    setAttempted(false);
  }
  function handleNext(): void {
    if (persistence?.readOnly === true) {
      if (position < snapshot.groups.length - 1) setGroupId(snapshot.groups[position + 1]?.id);
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
    if (position < snapshot.groups.length - 1) {
      if (persistence !== undefined) {
        persistence.onSave(
          {
            answers: snapshot.answers,
            language,
            groupId: snapshot.groups[position + 1]?.id ?? null,
          },
          false,
        );
        return;
      }
      setGroupId(snapshot.groups[position + 1]?.id);
      setAttempted(false);
      return;
    }
    const all = validate(runtime, snapshot);
    if (all.errors.size > 0) {
      const firstGroup = snapshot.groups.find((group) =>
        [...all.errors.keys()].some((id) => runtime.questions.get(id)?.source.groupId === group.id),
      );
      setGroupId(firstGroup?.id);
      return;
    }
    if (!cannotComplete) {
      if (persistence === undefined) setComplete(true);
      else
        persistence.onSave(
          { answers: snapshot.answers, language, groupId: currentGroup?.id ?? null },
          true,
        );
    }
  }
  function handleSaveDraft(): void {
    persistence?.onSave(
      { answers: snapshot.answers, language, groupId: currentGroup?.id ?? null },
      false,
    );
  }
  function handleDownload(): void {
    downloadAnswers(study, language, snapshot, persistence?.submitted ?? complete);
  }
  function handleReset(): void {
    setAnswers(new Map());
    setGroupId(study.groups[0]?.id);
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
  if (position === snapshot.groups.length - 1) nextLabel = "Valider la saisie locale";
  if (position === snapshot.groups.length - 1 && persistence !== undefined)
    nextLabel = "Envoyer mes réponses";
  if (position === snapshot.groups.length - 1 && persistence?.readOnly === true)
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
  if (persistence?.readOnly !== true && position === snapshot.groups.length - 1 && cannotComplete)
    nextDisabled = true;
  if (hasWelcome && !started)
    pageContent = <SurveyMessage message={welcome} completed={false} onContinue={handleStart} />;
  else if (complete)
    pageContent = (
      <SurveyMessage message={ending} completed={true} mode={messageMode} onContinue={handleEdit} />
    );
  else if (currentGroup !== undefined)
    pageContent = (
      <StudyPage
        runtime={runtime}
        mediaBaseUrl={mediaBaseUrl}
        snapshot={snapshot}
        group={currentGroup}
        language={language}
        position={position}
        validation={currentValidation}
        attempted={attempted}
        readOnly={readOnly}
        onAnswer={handleAnswer}
        onClear={handleClear}
        onLanguage={handleLanguageCode}
        previousDisabled={position <= 0 && !hasWelcome}
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
