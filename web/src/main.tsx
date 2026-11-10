import { render } from "preact";

import { App } from "./app";
import "./style.css";

const root = document.getElementById("app");
if (root === null) {
  throw new Error("Le conteneur de l’application est introuvable.");
}

render(<App />, root);
