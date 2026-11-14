import type { Answers } from "../survey/types.ts";
import type { NavigationSettings } from "../survey/navigation.ts";

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
  onSave: (draft: ViewerDraft, submit: boolean) => void;
}
