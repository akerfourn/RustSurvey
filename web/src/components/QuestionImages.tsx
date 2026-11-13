import type { SurveyQuestion } from "../survey/types.ts";
import { ObjectImage } from "./ObjectImage.tsx";

interface Props {
  question: SurveyQuestion;
  baseUrl: string | null;
  language: string;
}
export function QuestionImages({ question, baseUrl, language }: Props) {
  if (baseUrl === null) return null;
  const images = (question.media ?? []).filter(
    (image) => image.language === null || image.language === language,
  );
  return (
    <div className="question-images">
      {images.map((image) => (
        <figure key={`${image.id}-${image.language ?? "all"}`}>
          <ObjectImage src={`${baseUrl}/${encodeURIComponent(image.id)}`} alt={image.alt} />
        </figure>
      ))}
    </div>
  );
}
