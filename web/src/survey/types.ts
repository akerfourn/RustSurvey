export interface LocalizedTitle {
  language: string;
  value: string | null;
}
export interface GroupLabel {
  language: string;
  name: string | null;
  description: string | null;
}
export interface QuestionLabel {
  language: string;
  text: string | null;
  help: string | null;
}
export interface ChoiceLabel {
  language: string;
  text: string | null;
}
export interface QuestionAttribute {
  name: string;
  value: string;
  language: string | null;
}

export interface SurveyMessages {
  language: string;
  welcome: string | null;
  end: string | null;
}

export interface SurveyGroup {
  id: string;
  order: number;
  relevance: string;
  randomizationGroup: string;
  labels: GroupLabel[];
}

export interface SurveyQuestion {
  id: string;
  groupId: string;
  code: string;
  sourceType: string;
  theme?: string;
  family: string;
  mandatory: "no" | "soft" | "strict";
  order: number;
  relevance: string;
  randomizationGroup: string;
  validation: string;
  other: boolean;
  labels: QuestionLabel[];
  attributes: QuestionAttribute[];
}

export interface SurveySubquestion {
  id: string;
  parentId: string;
  groupId: string;
  code: string;
  scale: number;
  order: number;
  relevance: string;
  labels: QuestionLabel[];
}

export interface SurveyChoice {
  id: string;
  questionId: string;
  code: string;
  order: number;
  scale: number;
  labels: ChoiceLabel[];
}

export interface SurveyStudy {
  surveyId: string | null;
  baseLanguage: string | null;
  languages: string[];
  titles: LocalizedTitle[];
  showWelcome: boolean;
  messages: SurveyMessages[];
  groups: SurveyGroup[];
  questions: SurveyQuestion[];
  subquestions: SurveySubquestion[];
  answers: SurveyChoice[];
}

export interface SourceMetadata {
  surveyId: string | null;
  baseLanguage: string | null;
  languages: string[];
  titles: LocalizedTitle[];
}

export interface Diagnostic {
  severity: "error" | "warning";
  code: string;
  location: string;
  message: string;
}

export interface InspectionReport {
  sourceSchema: number;
  targetSchema: number;
  errors: number;
  warnings: number;
  omittedDiagnostics: number;
  checkedSections: string[];
  sections: { name: string; rows: number }[];
  diagnostics: Diagnostic[];
}

export interface Inspection {
  metadata: SourceMetadata;
  study: SurveyStudy | null;
  report: InspectionReport;
}

export type Answers = ReadonlyMap<string, string>;
