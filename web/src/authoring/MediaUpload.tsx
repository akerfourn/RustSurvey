import { useEffect, useRef, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { ApiError, message } from "./api.ts";
import { uploadMedia, type MediaRecord } from "./media-api.ts";
import { createId } from "./model.ts";

interface Props {
  study: string;
  csrf: string;
  disabled: boolean;
  resume: MediaRecord | null;
  onComplete: (media: MediaRecord) => void;
  onCancelResume: () => void;
  onExpired: () => void;
  onBusy: (busy: boolean) => void;
}

export function MediaUpload({
  study,
  csrf,
  disabled,
  resume,
  onComplete,
  onCancelResume,
  onExpired,
  onBusy,
}: Props) {
  const [file, setFile] = useState<File | null>(null);
  const [uploadId, setUploadId] = useState(createId);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const request = useRef<AbortController | null>(null);
  useEffect(
    () => () => {
      request.current?.abort();
      onBusy(false);
    },
    [onBusy],
  );
  useEffect(() => {
    setFile(null);
    setUploadId(resume?.id ?? createId());
    setError(null);
    if (input.current !== null) input.current.value = "";
  }, [resume?.id]);
  function handleFile(event: TargetedEvent<HTMLInputElement>): void {
    const selected = event.currentTarget.files?.[0] ?? null;
    if (resume !== null && selected !== null && selected.name !== resume.filename) {
      setError(`Sélectionner le même fichier, nommé « ${resume.filename} ».`);
      setFile(null);
      return;
    }
    setFile(selected);
    setUploadId(resume?.id ?? createId());
    setError(null);
  }
  async function handleUpload(): Promise<void> {
    if (file === null || busy) return;
    const controller = new AbortController();
    request.current = controller;
    setBusy(true);
    onBusy(true);
    setError(null);
    try {
      const media = await uploadMedia(study, uploadId, file, csrf, controller.signal);
      if (controller.signal.aborted) return;
      setFile(null);
      if (input.current !== null) input.current.value = "";
      onComplete(media);
    } catch (cause) {
      if (controller.signal.aborted) return;
      setError(message(cause));
      if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403)) onExpired();
    } finally {
      if (!controller.signal.aborted) {
        setBusy(false);
        onBusy(false);
      }
    }
  }
  let notice = null;
  let errorContent = null;
  if (resume !== null)
    notice = (
      <p className="notice">
        Reprise de « {resume.filename} » : sélectionner le fichier original.
        <button
          type="button"
          className="text-button"
          onClick={onCancelResume}
          disabled={busy || disabled}
        >
          Choisir un nouvel envoi
        </button>
      </p>
    );
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  return (
    <div>
      {notice}
      <fieldset disabled={disabled || busy}>
        <label>
          Choisir une image
          <input ref={input} type="file" accept="image/png,image/jpeg" onChange={handleFile} />
        </label>
        <button type="button" onClick={handleUpload} disabled={file === null}>
          Enregistrer l’image
        </button>
      </fieldset>
      {errorContent}
    </div>
  );
}
