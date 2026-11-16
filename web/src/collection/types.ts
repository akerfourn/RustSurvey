import type { Answers } from "../survey/types.ts";
import type { NavigationSettings } from "../survey/navigation.ts";
import type { Acknowledgement } from "./api.ts";

export interface ViewerDraft {
  answers: Answers;
  language: string;
  groupId: string | null;
  pageId?: string | undefined;
}

export interface ViewerPersistence extends ViewerDraft {
  profile?: string | undefined;
  navigation: NavigationSettings | null;
  submitted: boolean;
  hasProgress: boolean;
  readOnly: boolean;
  disabled: boolean;
  saving: boolean;
  blocked: boolean;
  uncertain: boolean;
  saveFailed: boolean;
  onSave: (draft: ViewerDraft, submit: boolean, background: boolean) => Promise<SavedDraft | null>;
}

export interface SavedDraft extends ViewerDraft {
  acknowledgement: Acknowledgement;
}
