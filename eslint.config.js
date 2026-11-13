import eslint from "@eslint/js";
import prettier from "eslint-config-prettier";
import globals from "globals";
import typescript from "typescript-eslint";

export default [
  {
    ignores: [
      "target/**",
      "**/node_modules/**",
      "web/dist/**",
      "coverage/**",
      ".data/**",
      ".private/**",
      ".surveys-tests/**",
    ],
  },
  eslint.configs.recommended,
  ...typescript.configs.recommended,
  {
    files: ["**/*.{js,mjs,ts,tsx}"],
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
  },
  prettier,
];
