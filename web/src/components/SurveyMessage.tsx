import { useEffect, useRef } from "preact/hooks";
import type { RenderedText } from "../survey/text.ts";

interface SurveyMessageProps {
  message: RenderedText;
  completed: boolean;
  onContinue: () => void;
}

export function SurveyMessage({ message, completed, onContinue }: SurveyMessageProps) {
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
