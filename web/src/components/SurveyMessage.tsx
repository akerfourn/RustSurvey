import { useEffect, useRef } from "preact/hooks";
import type { RenderedText } from "../survey/text.ts";

interface SurveyMessageProps {
  message: RenderedText;
  completed: boolean;
  onContinue: () => void;
  mode?: "local" | "submitted" | "closed";
}

export function SurveyMessage({
  message,
  completed,
  onContinue,
  mode = "local",
}: SurveyMessageProps) {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, [completed]);
  let title = "Bienvenue";
  let buttonLabel = "Commencer";
  let status = null;
  if (completed) {
    title = "Saisie locale validée";
    buttonLabel = "Modifier mes réponses";
    status = (
      <p className="notice success" role="status">
        Les réponses restent uniquement dans cet onglet. Vous pouvez les télécharger ci-dessous.
      </p>
    );
    if (mode === "submitted") {
      title = "Réponses envoyées";
      buttonLabel = "Consulter mes réponses";
      status = (
        <p className="notice success" role="status">
          Vos réponses ont été enregistrées et envoyées. Elles ne peuvent plus être modifiées.
        </p>
      );
    } else if (mode === "closed") {
      title = "Campagne fermée";
      buttonLabel = "Consulter mes réponses";
      status = (
        <p className="notice" role="status">
          Vos réponses enregistrées restent consultables. Cette campagne n’accepte plus de
          modifications.
        </p>
      );
    }
  }
  return (
    <section className="survey-message">
      <h3 ref={heading} tabIndex={-1}>
        {title}
      </h3>
      {status}
      <p className="survey-message-text">{message.text}</p>
      <button type="button" onClick={onContinue}>
        {buttonLabel}
      </button>
    </section>
  );
}
