import { ObjectImage } from "../components/ObjectImage.tsx";
import type { MediaRecord } from "./media-api.ts";
import type { Question } from "./model.ts";

interface Props {
  study: string;
  media: MediaRecord;
  question: Question | undefined;
  disabled: boolean;
  busy: boolean;
  onAttach: (media: MediaRecord) => void;
  onResume: (media: MediaRecord) => void;
}
export function MediaCard({ study, media, question, disabled, busy, onAttach, onResume }: Props) {
  function handleAttach(): void {
    onAttach(media);
  }
  function handleResume(): void {
    onResume(media);
  }
  const attached =
    question?.media.some((image) => image.id === media.id && image.language === null) ?? false;
  let thumbnail = <p>Envoi incomplet.</p>;
  let resume = null;
  if (media.state === "ready")
    thumbnail = (
      <ObjectImage
        src={`/api/admin/studies/${encodeURIComponent(study)}/media/${encodeURIComponent(media.id)}`}
        alt={media.filename}
      />
    );
  else
    resume = (
      <button
        type="button"
        className="secondary"
        onClick={handleResume}
        disabled={busy || disabled}
      >
        Reprendre cet envoi
      </button>
    );
  return (
    <li className="media-card">
      {thumbnail}
      <p>
        {media.filename} · {Math.ceil(media.size / 1024)} Kio
      </p>
      {resume}
      <button
        type="button"
        className="secondary"
        onClick={handleAttach}
        disabled={
          disabled ||
          question === undefined ||
          question.media.length >= 8 ||
          attached ||
          media.state !== "ready"
        }
      >
        Associer à la question
      </button>
    </li>
  );
}
