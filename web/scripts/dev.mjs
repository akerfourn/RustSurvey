import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { cargoEnvironment } from "../../scripts/cargo-environment.mjs";

const projectDirectory = fileURLToPath(new URL("../../", import.meta.url));
const webDirectory = fileURLToPath(new URL("../", import.meta.url));
const readyPrefix = "RUSTSURVEY_DEV_API_URL=";
const backendOnly = process.argv[2] === "--backend-only";
let backend;
let frontend;
let shutdownTask;
let stopping = false;

function terminateBackend(signal) {
  const child = backend?.child;
  if (child?.pid === undefined) return;
  try {
    if (process.platform === "win32") child.kill(signal);
    else process.kill(-child.pid, signal);
  } catch (error) {
    if (error.code !== "ESRCH") console.error("Cannot stop backend:", error.message);
  }
}

async function closeServices(code) {
  stopping = true;
  process.exitCode = code;
  terminateBackend("SIGTERM");
  const forceTermination = setTimeout(() => terminateBackend("SIGKILL"), 5000);
  forceTermination.unref();
  try {
    await Promise.all([frontend?.close(), backend?.closed]);
  } finally {
    clearTimeout(forceTermination);
  }
}

function shutdown(code) {
  if (shutdownTask === undefined) shutdownTask = closeServices(code);
  return shutdownTask;
}

function launchBackend() {
  const child = spawn("cargo", ["run", "-p", "rustsurvey-server", "--locked"], {
    cwd: projectDirectory,
    detached: process.platform !== "win32",
    env: cargoEnvironment({ ...process.env, RUSTSURVEY_DEV_READY: "1" }),
    stdio: ["ignore", "pipe", "inherit"],
  });
  let resolveReady;
  let rejectReady;
  const ready = new Promise((resolve, reject) => {
    resolveReady = resolve;
    rejectReady = reject;
  });
  child.once("error", (error) => {
    if (error.code === "ENOENT") {
      rejectReady(
        new Error(
          "Cargo introuvable. Installez Rust avec rustup ou ajoutez son dossier bin au PATH.",
        ),
      );
      return;
    }
    rejectReady(error);
  });
  const closed = new Promise((resolve) => {
    child.once("close", (code) => {
      rejectReady(new Error(`Backend stopped (exit ${code ?? "signal"}).`));
      resolve();
      if (!stopping) {
        let exitCode = 1;
        if (code !== null && code !== 0) exitCode = code;
        void shutdown(exitCode);
      }
    });
  });
  const lines = createInterface({ input: child.stdout });
  lines.on("line", (line) => {
    if (!line.startsWith(readyPrefix)) {
      console.log(line);
      return;
    }
    try {
      const url = new URL(line.slice(readyPrefix.length));
      if (
        url.protocol !== "http:" ||
        url.port === "" ||
        url.pathname !== "/" ||
        url.username !== "" ||
        url.password !== ""
      ) {
        throw new Error("Backend returned an invalid development URL.");
      }
      resolveReady(url.origin);
    } catch (error) {
      rejectReady(error);
    }
  });
  return { child, closed, ready };
}

async function main() {
  process.once("SIGINT", () => {
    void shutdown(130);
  });
  process.once("SIGTERM", () => {
    void shutdown(143);
  });
  try {
    let target;
    if (!backendOnly) target = process.env.API_PROXY_TARGET;
    if (target === undefined) {
      backend = launchBackend();
      target = await backend.ready;
    }
    if (stopping) return;
    if (backendOnly) {
      console.log(`\nRustSurvey API: ${target}`);
      return;
    }
    const { createServer } = await import("vite");
    frontend = await createServer({
      root: webDirectory,
      server: { proxy: { "/api": target } },
    });
    if (stopping) {
      await frontend.close();
      return;
    }
    await frontend.listen();
    console.log(`\nRustSurvey API: ${target}`);
    frontend.printUrls();
  } catch (error) {
    if (!stopping) {
      console.error(error.message);
      await shutdown(1);
    }
  }
}

await main();
