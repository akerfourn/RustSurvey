import { useEffect, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { ApiError, message } from "./api.ts";
import { listMedia, type MediaRecord } from "./media-api.ts";
import { type Definition } from "./model.ts";
import { MediaCard } from "./MediaCard.tsx";
import { MediaUpload } from "./MediaUpload.tsx";

interface Props {
  id: string;
  csrf: string;
  definition: Definition;
  disabled: boolean;
  onChange: (definition: Definition) => void;
  onExpired: () => void;
}
function LibraryBody({ id, csrf, definition, disabled, onChange, onExpired }: Props) {
  function reportError(cause: unknown): void {
    setError(message(cause));
    if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403)) onExpired();
  }
  const [items, setItems] = useState<MediaRecord[]>([]);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [refresh, setRefresh] = useState(0);
  const [resume, setResume] = useState<MediaRecord | null>(null);
  const [target, setTarget] = useState("");
  const [uploadBusy, setUploadBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    async function load(): Promise<void> {
      try {
        const page = await listMedia(id, offset, controller.signal);
        if (!controller.signal.aborted) {
          setItems(page.items);
          setHasMore(page.hasMore);
        }
      } catch (cause) {
        if (!controller.signal.aborted) reportError(cause);
      }
    }
    void load();
    return () => controller.abort();
  }, [id, offset, refresh, csrf, onExpired]);
  function handleTarget(event: TargetedEvent<HTMLSelectElement>): void {
    setTarget(event.currentTarget.value);
  }
  function handleNext(): void {
    setOffset(offset + 50);
  }
  function handlePrevious(): void {
    setOffset(Math.max(0, offset - 50));
  }
  function handleComplete(media: MediaRecord): void {
    setNotice(
      `Image « ${media.filename} » enregistrée. Associez-la à une question puis enregistrez le questionnaire.`,
    );
    setResume(null);
    setOffset(0);
    setRefresh((value) => value + 1);
  }
  function handleCancelResume(): void {
    setResume(null);
  }
  const questions = definition.groups.flatMap((group) => group.questions);
  const question = questions.find((item) => item.id === target);
  function handleAttach(media: MediaRecord): void {
    if (question === undefined || question.media.length >= 8 || media.state !== "ready") return;
    onChange({
      ...definition,
      groups: definition.groups.map((group) => ({
        ...group,
        questions: group.questions.map((item) => {
          if (item.id !== target) return item;
          return {
            ...item,
            media: [...item.media, { id: media.id, alt: media.filename, language: null }],
          };
        }),
      })),
    });
    setNotice("Image associée au brouillon. Enregistrez pour la rendre publique.");
  }
  const cards = items.map((media) => (
    <MediaCard
      key={media.id}
      study={id}
      media={media}
      question={question}
      disabled={disabled}
      busy={uploadBusy}
      onAttach={handleAttach}
      onResume={setResume}
    />
  ));
  let errorContent = null;
  let noticeContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  if (notice !== null)
    noticeContent = (
      <p className="notice" role="status">
        {notice}
      </p>
    );
  return (
    <div>
      <p>
        Images PNG/JPEG, 8 Mio maximum. Les fichiers sont privés tant qu’ils ne sont pas associés à
        une révision courante enregistrée. Les images des anciennes révisions sont conservées.
      </p>
      <MediaUpload
        study={id}
        csrf={csrf}
        disabled={disabled}
        resume={resume}
        onComplete={handleComplete}
        onCancelResume={handleCancelResume}
        onExpired={onExpired}
        onBusy={setUploadBusy}
      />
      {errorContent}
      {noticeContent}
      <label>
        Question à illustrer
        <select value={target} onChange={handleTarget} disabled={disabled}>
          <option value="">Choisir une question</option>
          {questions.map((item) => (
            <option key={item.id} value={item.id}>
              {item.code} · {item.title.slice(0, 80)}
            </option>
          ))}
        </select>
      </label>
      <ul className="media-grid">{cards}</ul>
      <div className="button-row">
        <button
          type="button"
          className="text-button"
          onClick={handlePrevious}
          disabled={offset === 0}
        >
          Précédentes
        </button>
        <button type="button" className="text-button" onClick={handleNext} disabled={!hasMore}>
          Suivantes
        </button>
      </div>
    </div>
  );
}

export function MediaLibrary(props: Props) {
  const [open, setOpen] = useState(false);
  function handleToggle(event: TargetedEvent<HTMLDetailsElement>): void {
    setOpen(event.currentTarget.open);
  }
  let content = null;
  if (open) content = <LibraryBody {...props} />;
  return (
    <details className="panel" onToggle={handleToggle}>
      <summary>Médiathèque du questionnaire</summary>
      {content}
    </details>
  );
}
