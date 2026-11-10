import { useMemo, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import {
  createRuntime,
  evaluateSnapshot,
  runtimeIssues,
  type Snapshot,
  type SurveyRuntime,
} from "../survey/engine.ts";
import { localized, plainText } from "../survey/text.ts";
import type { Inspection, SurveyStudy } from "../survey/types.ts";
import { validate, type Validation } from "../survey/validation.ts";
import { QuestionCard } from "./QuestionCard.tsx";

interface StudyViewerProps {
  study: SurveyStudy;
  inspection: Inspection;
}

function firstErrorGroup(
  runtime: SurveyRuntime,
  validation: Validation,
  snapshot: Snapshot,
): string | undefined {
  return snapshot.groups.find((group) =>
    [...validation.errors.keys()].some(
      (id) => runtime.questions.get(id)?.source.groupId === group.id,
    ),
  )?.id;
}

function downloadAnswers(
  study: SurveyStudy,
  language: string,
  snapshot: Snapshot,
  complete: boolean,
): void {
  const content = JSON.stringify(
    {
      format: "rustsurvey-local-preview-v1",
      surveyId: study.surveyId,
      language,
      complete,
      answers: Object.fromEntries(snapshot.answers),
    },
    null,
    2,
  );
  const url = URL.createObjectURL(new Blob([content], { type: "application/json" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "rustsurvey-local-answers.json";
  anchor.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function StudyViewer({ study, inspection }: StudyViewerProps) {
  const runtime = useMemo(() => createRuntime(study), [study]);
  const [answers, setAnswers] = useState(new Map<string, string>());
  const [language, setLanguage] = useState(study.baseLanguage ?? study.languages[0] ?? "fr");
  const [groupId, setGroupId] = useState<string | undefined>(study.groups[0]?.id);
  const [attempted, setAttempted] = useState(false);
  const [complete, setComplete] = useState(false);
  const snapshot = useMemo(() => evaluateSnapshot(runtime, answers), [runtime, answers]);
  const currentGroup = snapshot.groups.find((group) => group.id === groupId) ?? snapshot.groups[0];
  const position = snapshot.groups.findIndex((group) => group.id === currentGroup?.id);
  const currentValidation = validate(runtime, snapshot, currentGroup?.id);
  const issues = runtimeIssues(runtime, snapshot);
  for (const section of inspection.report.sections) {
    if (
      ["conditions", "defaultvalues", "quota", "quota_members", "quota_languagesettings"].includes(
        section.name,
      ) &&
      section.rows > 0
    ) {
      issues.push(`Section ${section.name} : comportement non pris en charge par cet aperçu.`);
    }
  }
  const readOnly = inspection.report.errors > 0 || runtime.issues.length > 0;
  const cannotComplete = readOnly || issues.length > 0;

  function handleAnswer(field: string, value: string): void {
    setAnswers((previous) => {
      const updated = new Map(previous);
      updated.set(field, value);
      return evaluateSnapshot(runtime, updated).answers;
    });
    setComplete(false);
  }
  function handleLanguage(event: TargetedEvent<HTMLSelectElement>): void {
    setLanguage(event.currentTarget.value);
  }
  function handlePrevious(): void {
    setGroupId(snapshot.groups[position - 1]?.id);
    setAttempted(false);
    setComplete(false);
  }
  function handleNext(): void {
    setAttempted(true);
    if (currentValidation.errors.size > 0) return;
    if (currentValidation.warnings.size > 0 && !attempted) return;
    if (position < snapshot.groups.length - 1) {
      setGroupId(snapshot.groups[position + 1]?.id);
      setAttempted(false);
      return;
    }
    const all = validate(runtime, snapshot);
    if (all.errors.size > 0) {
      setGroupId(firstErrorGroup(runtime, all, snapshot));
      return;
    }
    if (!cannotComplete) setComplete(true);
  }
  function handleDownload(): void {
    downloadAnswers(study, language, snapshot, complete);
  }
  function handleReset(): void {
    setAnswers(new Map());
    setGroupId(study.groups[0]?.id);
    setAttempted(false);
    setComplete(false);
  }
  const languageOptions = study.languages.map((value) => (
    <option key={value} value={value}>
      {value.toUpperCase()}
    </option>
  ));
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
  let readOnlyContent = null;
  if (readOnly)
    readOnlyContent = (
      <p className="notice error">
        La structure contient des erreurs ou dépasse les limites de l’aperçu. La saisie est
        désactivée.
      </p>
    );
  let pageContent = (
    <p className="notice">Aucune question pertinente à afficher pour les réponses actuelles.</p>
  );
  let nextLabel = "Groupe suivant";
  if (position === snapshot.groups.length - 1) nextLabel = "Valider la saisie locale";
  if (attempted && currentValidation.errors.size === 0 && currentValidation.warnings.size > 0) {
    nextLabel = "Continuer sans les réponses recommandées";
  }
  if (currentGroup !== undefined) {
    const groupLabel = localized(currentGroup.labels, language, study.baseLanguage);
    const questions = [...runtime.questions.values()].filter(
      (question) =>
        question.source.groupId === currentGroup.id && snapshot.visible.has(question.source.id),
    );
    const questionContent = questions.map((question) => {
      let errors: string[] = [];
      let warnings: string[] = [];
      if (attempted) {
        errors = currentValidation.errors.get(question.source.id) ?? [];
        warnings = currentValidation.warnings.get(question.source.id) ?? [];
      }
      return (
        <QuestionCard
          key={question.source.id}
          question={question}
          answers={snapshot.answers}
          language={language}
          baseLanguage={study.baseLanguage}
          fields={runtime.fields}
          errors={errors}
          warnings={warnings}
          readOnly={readOnly}
          onAnswer={handleAnswer}
        />
      );
    });
    let validationContent = null;
    if (attempted && currentValidation.errors.size > 0)
      validationContent = (
        <p className="notice error" role="alert">
          Corrigez les réponses signalées avant de continuer.
        </p>
      );
    pageContent = (
      <div>
        <div className="group-heading">
          <p className="eyebrow">
            Groupe {position + 1} sur {snapshot.groups.length}
          </p>
          <h3>{plainText(groupLabel?.name) || `Groupe ${currentGroup.id}`}</h3>
          <p className="muted">{plainText(groupLabel?.description)}</p>
        </div>
        {questionContent}
        {validationContent}
        <nav className="viewer-navigation" aria-label="Parcours du questionnaire">
          <button
            type="button"
            className="secondary"
            disabled={position <= 0}
            onClick={handlePrevious}
          >
            Précédent
          </button>
          <button
            type="button"
            disabled={readOnly || (position === snapshot.groups.length - 1 && cannotComplete)}
            onClick={handleNext}
          >
            {nextLabel}
          </button>
        </nav>
      </div>
    );
  }
  let completionContent = null;
  if (complete)
    completionContent = (
      <p className="notice success" role="status">
        Saisie locale validée. Les réponses sont uniquement dans cet onglet ; vous pouvez les
        télécharger ci-dessous.
      </p>
    );
  return (
    <section className="viewer panel" aria-labelledby="viewer-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Saisie dans le navigateur</p>
          <h2 id="viewer-title">Aperçu de l’étude</h2>
        </div>
        <label className="language-control">
          Langue
          <select value={language} onChange={handleLanguage}>
            {languageOptions}
          </select>
        </label>
      </div>
      <p className="notice">
        Aperçu partiel, sans compte. Les réponses restent dans cet onglet et disparaissent au
        rechargement. Aucun enregistrement serveur.
      </p>
      {readOnlyContent}
      {issueContent}
      {pageContent}
      {completionContent}
      <div className="local-actions">
        <button type="button" className="secondary" onClick={handleDownload}>
          Télécharger les réponses (JSON)
        </button>
        <button type="button" className="text-button" onClick={handleReset}>
          Recommencer la saisie
        </button>
      </div>
    </section>
  );
}
