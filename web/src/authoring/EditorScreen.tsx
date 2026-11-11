import { ImportDetails } from "./ImportDetails.tsx";
import { useMemo, useState } from "preact/hooks";
import type { Session, ValidationIssue } from "./model.ts";
import { toViewer } from "./model.ts";
import { useEditor } from "./useEditor.ts";
import { DefinitionEditor } from "./DefinitionEditor.tsx";
import { LoginPanel } from "./LoginPanel.tsx";
import { StudyViewer } from "../components/StudyViewer.tsx";
import { ViewerBoundary } from "../components/ViewerBoundary.tsx";

interface EditorScreenProps {
  id: string | null;
  session: Session;
  onSession: (session: Session) => void;
}
function issueLabel(issue: ValidationIssue): string {
  const group = /groups\[(\d+)\]/.exec(issue.path);
  const question = /questions\[(\d+)\]/.exec(issue.path);
  const choice = /choices\[(\d+)\]/.exec(issue.path);
  const parts: string[] = [];
  if (group !== null) parts.push(`Groupe ${Number(group[1]) + 1}`);
  if (question !== null) parts.push(`question ${Number(question[1]) + 1}`);
  if (choice !== null) parts.push(`choix ${Number(choice[1]) + 1}`);
  if (parts.length === 0) parts.push("Questionnaire");
  return `${parts.join(" · ")} : ${issue.message}`;
}
export function EditorScreen({ id, session, onSession }: EditorScreenProps) {
  const editor = useEditor(id, session.csrfToken);
  const [preview, setPreview] = useState(false);
  const study = useMemo(() => toViewer(editor.definition, id), [editor.definition, id]);
  function handleSave(): void {
    void editor.save();
  }
  function handlePreview(): void {
    setPreview(!preview);
  }
  function handleBack(): void {
    if (!editor.dirty || window.confirm("Quitter sans enregistrer les modifications ?"))
      window.location.hash = "/admin";
  }
  function handleLogin(updated: Session): void {
    onSession(updated);
    editor.setNeedsLogin(false);
  }
  let error = null;
  if (editor.error !== null)
    error = (
      <div className="notice error" role="alert">
        <p>{editor.error}</p>
        <ul>
          {editor.issues.map((issue, index) => (
            <li key={`${issue.path}-${index}`}>{issueLabel(issue)}</li>
          ))}
        </ul>
      </div>
    );
  let login = null;
  if (editor.needsLogin) login = <LoginPanel configured={true} onLogin={handleLogin} />;
  let publicLink = null;
  if (id !== null)
    publicLink = (
      <a
        className="button-link secondary"
        href={`#/studies/${id}`}
        target="_blank"
        rel="noreferrer"
      >
        Ouvrir le questionnaire public
      </a>
    );
  let status = "Brouillon non enregistré";
  if (editor.revision > 0) status = `Version ${editor.revision} enregistrée`;
  if (editor.dirty) status = "Modifications non enregistrées";
  if (editor.saving) status = "Enregistrement…";
  let previewLabel = "Aperçu";
  if (preview) previewLabel = "Revenir à l’éditeur";
  let content = <p role="status">Chargement du questionnaire…</p>;
  if (editor.ready) {
    content = (
      <fieldset className="editor-body" disabled={editor.saving}>
        <DefinitionEditor
          definition={editor.definition}
          readOnlyContent={editor.source !== null}
          onChange={editor.change}
        />
      </fieldset>
    );
    if (preview)
      content = (
        <ViewerBoundary key={`preview-${editor.revision}`}>
          <StudyViewer study={study} report={editor.source?.report} debug={true} />
        </ViewerBoundary>
      );
  }
  let reload = null;
  if (id !== null)
    reload = (
      <button
        type="button"
        className="text-button"
        onClick={editor.reloadSaved}
        disabled={editor.saving}
      >
        Recharger la version enregistrée
      </button>
    );
  return (
    <div className="authoring-editor">
      <section className="panel editor-toolbar">
        <div>
          <h1>Éditeur de questionnaire</h1>
          <p role="status">{status}</p>
          <p className="muted">
            Le questionnaire devient public dès son premier enregistrement. Les réponses restent
            locales.
          </p>
        </div>
        <div className="button-row">
          <button type="button" className="secondary" onClick={handleBack} disabled={editor.saving}>
            Mes questionnaires
          </button>
          <button
            type="button"
            className="secondary"
            onClick={handlePreview}
            disabled={!editor.ready}
          >
            {previewLabel}
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={!editor.ready || editor.saving || (!editor.dirty && editor.revision > 0)}
          >
            Enregistrer
          </button>
          {publicLink}
          {reload}
        </div>
        {error}
      </section>
      {login}
      <ImportDetails id={id} source={editor.source} />
      {content}
    </div>
  );
}
