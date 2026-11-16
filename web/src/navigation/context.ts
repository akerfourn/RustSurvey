import { createContext } from "preact";
import type { Navigation } from "./controller.ts";

export const NavigationContext = createContext<Navigation | null>(null);
