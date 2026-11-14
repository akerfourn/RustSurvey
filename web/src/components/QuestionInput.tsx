import type { TargetedEvent } from "preact";
import { isChoiceArray, isMatrix } from "../survey/question-kinds.ts";
import type { RuntimeQuestion } from "../survey/engine.ts";
import type { Answers } from "../survey/types.ts";
import { ArrayInput } from "./ArrayInput.tsx";
import { ChoiceInput } from "./ChoiceInput.tsx";
import { MultipleInput } from "./MultipleInput.tsx";
import { RankingInput } from "./RankingInput.tsx";
import { TextControl } from "./TextControl.tsx";

export interface QuestionInputProps {
  question: RuntimeQuestion;
  answers: Answers;
  language: string;
  baseLanguage: string | null;
  fields: ReadonlyMap<string, string>;
  activeFields: ReadonlySet<string>;
  activeChoices?: ReadonlySet<string> | undefined;
  otherVisible?: boolean;
  languages: readonly string[];
  onLanguage: (language: string) => void;
  onAnswer: (field: string, value: string) => void;
}

export function QuestionInput(props: QuestionInputProps) {
  const { question, answers, onAnswer, onLanguage, languages, language } = props;
  const type = question.source.sourceType;
  function handleLanguage(event: TargetedEvent<HTMLSelectElement>): void {
    const value = event.currentTarget.value;
    onAnswer(question.source.code, value);
    onLanguage(value);
  }
  if (isChoiceArray(type) || isMatrix(type)) return <ArrayInput {...props} />;
  if (["L", "!", "5", "Y", "G", "O"].includes(type)) return <ChoiceInput {...props} />;
  if (["M", "P", "Q", "K"].includes(type)) return <MultipleInput {...props} />;
  if (type === "R") return <RankingInput {...props} />;
  if (type === "*")
    return (
      <output className="computed-answer" aria-label={`Résultat ${question.source.code}`}>
        {answers.get(question.source.code) ?? ""}
      </output>
    );
  if (type === "I")
    return (
      <select value={language} aria-label="Langue du questionnaire" onChange={handleLanguage}>
        {languages.map((item) => (
          <option key={item} value={item}>
            {item.toUpperCase()}
          </option>
        ))}
      </select>
    );
  return <TextControl {...props} field={question.source.code} label={question.source.code} />;
}
