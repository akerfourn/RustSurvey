import { useMemo, useState } from "preact/hooks";
import { toViewer } from "../authoring/model.ts";
import { StudyViewer } from "../components/StudyViewer.tsx";
import { ViewerBoundary } from "../components/ViewerBoundary.tsx";
import { resumeFragment, type Participation } from "./api.ts";
import type { SavedDraft, ViewerDraft } from "./types.ts";
import { useResponseWriter } from "./useResponseWriter.ts";

interface Props {
  initial: Participation;
  token: string;
  invitationToken: string | undefined;
  onReload: () => void;
}

export function ParticipationForm({ initial, token, invitationToken, onReload }: Props) {
  const writer = useResponseWriter(initial, token, invitationToken);
  const stored = writer.saved;
  const [editing, setEditing] = useState(false);
  const study = useMemo(() => toViewer(initial.definition, initial.run.studyId), [initial]);
  async function handleSave(
    draft: ViewerDraft,
    submit: boolean,
    background: boolean,
  ): Promise<SavedDraft | null> {
    const result = await writer.save(draft, submit, background);
    if (result?.acknowledgement.status === "submitted") setEditing(false);
    return result;
  }
  function handleEdit(): void {
    setEditing(true);
  }
  function handleReload(): void {
    if (
      window.confirm(
        "Recharger remplacera les modifications non enregistrées par la réponse conservée sur le serveur.",
      )
    )
      onReload();
  }
  let errorContent = null;
  if (writer.error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {writer.error}
      </p>
    );
  let status = "Le dernier enregistrement a été confirmé par le serveur.";
  if (stored.version === 0)
    status = "La participation est prête. Les modifications seront enregistrées automatiquement.";
  if (writer.busy) status = "Enregistrement…";
  if (writer.uncertain)
    status =
      "Enregistrement non confirmé. Utilisez le bouton de sauvegarde pour réessayer avant de poursuivre.";
  if (editing && stored.status === "submitted")
    status = "Les modifications remplaceront cette réponse lors du prochain enregistrement.";
  let editButton = null;
  if (
    stored.status === "submitted" &&
    stored.run.isOpen &&
    stored.run.participationPolicy.mode === "invitation" &&
    stored.run.participationPolicy.allowEditAfterSubmit &&
    !editing
  ) {
    editButton = (
      <button
        type="button"
        onClick={handleEdit}
        disabled={writer.busy || writer.uncertain || writer.blocked}
      >
        Modifier cette réponse
      </button>
    );
  }
  let stateNotice = null;
  if (stored.status === "submitted")
    stateNotice = <p className="notice">Vos réponses ont été envoyées.</p>;
  else if (!stored.run.isOpen)
    stateNotice = (
      <p className="notice">
        La campagne est fermée. Vos réponses enregistrées restent consultables.
      </p>
    );
  let groupId = stored.checkpoint.groupId;
  let pageId = stored.checkpoint.pageId;
  if (editing && stored.status === "submitted") {
    groupId = null;
    pageId = undefined;
  }
  const resume = `${window.location.origin}${window.location.pathname}${resumeFragment(stored.id, token, invitationToken)}`;
  return (
    <div>
      <section className="panel">
        <h1>{stored.definition.title}</h1>
        <label>
          Lien privé de reprise, à conserver
          <input type="url" readOnly value={resume} />
        </label>
        <p>
          Ce lien donne accès à vos réponses. Gardez-le privé ; il peut aussi contenir votre droit
          d’invitation.
        </p>
        {stateNotice}
        <p role="status">{status}</p>
        {errorContent}
        <ul>
          {writer.issues.map((issue, index) => (
            <li key={`${index}:${issue.path}`}>{issue.message}</li>
          ))}
        </ul>
        <div className="button-row">
          {editButton}
          <button type="button" className="secondary" onClick={handleReload} disabled={writer.busy}>
            Recharger la réponse enregistrée
          </button>
        </div>
      </section>
      <ViewerBoundary key={`${stored.id}:${editing}`}>
        <StudyViewer
          study={study}
          publishedRevision={stored.run.revision}
          persistence={{
            profile: stored.run.profile,
            answers: new Map(Object.entries(stored.answers)),
            language: stored.checkpoint.language,
            groupId,
            pageId,
            navigation: stored.run.navigation,
            submitted: stored.status === "submitted" && !editing,
            hasProgress: stored.version > 0 || !stored.run.isOpen,
            readOnly: (stored.status === "submitted" && !editing) || !stored.run.isOpen,
            disabled:
              (writer.busy && !writer.backgroundSaving) || writer.uncertain || writer.blocked,
            saving: writer.busy,
            blocked: writer.blocked,
            uncertain: writer.uncertain,
            saveFailed: writer.error !== null,
            onSave: handleSave,
          }}
        />
      </ViewerBoundary>
    </div>
  );
}
