import type { Answers } from "../survey/types.ts";

export interface ViewerDraft {
  answers: Answers;
  language: string;
  groupId: string | null;
}

export interface ViewerPersistence extends ViewerDraft {
  submitted: boolean;
  hasProgress: boolean;
  readOnly: boolean;
  disabled: boolean;
  onSave: (draft: ViewerDraft, submit: boolean) => void;
}
