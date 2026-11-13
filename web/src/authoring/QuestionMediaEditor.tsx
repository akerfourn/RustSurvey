import type { TargetedEvent } from "preact";
import type { Question } from "./model.ts";

export function QuestionMediaEditor({
  question,
  onChange,
}: {
  question: Question;
  onChange: (question: Question) => void;
}) {
  if (question.media.length === 0) return null;
  const rows = question.media.map((image, index) => {
    function handleAlt(event: TargetedEvent<HTMLInputElement>): void {
      const alt = event.currentTarget.value;
      onChange({
        ...question,
        media: question.media.map((item, position) => {
          if (position === index) return { ...item, alt };
          return item;
        }),
      });
    }
    function handleRemove(): void {
      onChange({ ...question, media: question.media.filter((_, position) => position !== index) });
    }
    return (
      <div className="choice-editor-row" key={`${image.id}-${image.language ?? "all"}`}>
        <label>
          Description de l’image {index + 1}
          <input value={image.alt} maxLength={1000} onInput={handleAlt} />
        </label>
        <button type="button" className="text-button" onClick={handleRemove}>
          Dissocier
        </button>
      </div>
    );
  });
  return (
    <fieldset>
      <legend>Images associées</legend>
      {rows}
      <p className="muted">
        Ajouter une image depuis la médiathèque. Dissocier conserve le fichier et l’historique.
      </p>
    </fieldset>
  );
}
