import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, realpath, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { createInterface } from "node:readline";

export class DisposableServer {
  child;
  base;
  directory;
  cookie = "";
  csrf = "";
  #binary;
  #queueCapacity;
  #assetsDirectory;
  #interrupted = false;

  constructor(binary, queueCapacity, { assetsDirectory } = {}) {
    this.#binary = binary;
    this.#queueCapacity = queueCapacity;
    this.#assetsDirectory = assetsDirectory;
  }

  interrupt() {
    this.#interrupted = true;
    this.child?.kill("SIGTERM");
  }

  async start() {
    if (this.#interrupted) throw new Error("Benchmark interrupted.");
    this.directory ??= await mkdtemp(join(tmpdir(), "rustsurvey-performance-"));
    const environment = {
      ...process.env,
      BIND_ADDRESS: "127.0.0.1:0",
      RUSTSURVEY_DEV_READY: "1",
      OBJECT_STORE_BACKEND: "file",
      OBJECT_STORE_DIRECTORY: ".data/media",
      ADMIN_USERNAME: "admin",
      ADMIN_PASSWORD: "synthetic-benchmark-password",
      ADMIN_COOKIE_SECURE: "false",
      DEV_ALLOW_WEAK_PASSWORDS: "false",
      COLLECTION_WRITE_QUEUE_CAPACITY: String(this.#queueCapacity),
    };
    delete environment.DATABASE_URL;
    if (this.#assetsDirectory === undefined) delete environment.STATIC_ASSETS_DIRECTORY;
    else environment.STATIC_ASSETS_DIRECTORY = this.#assetsDirectory;
    this.child = spawn(this.#binary, [], {
      cwd: this.directory,
      env: environment,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    this.child.stderr.resume();
    this.base = await new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error("Benchmark server startup timed out.")),
        15000,
      );
      const lines = createInterface({ input: this.child.stdout });
      lines.on("line", (line) => {
        if (line.startsWith("RUSTSURVEY_DEV_API_URL=")) {
          clearTimeout(timer);
          resolve(line.slice("RUSTSURVEY_DEV_API_URL=".length));
        }
      });
      this.child.once("error", () => {
        clearTimeout(timer);
        reject(new Error("Cannot launch the release server."));
      });
      this.child.once("exit", () => {
        clearTimeout(timer);
        reject(new Error("Benchmark server exited before readiness."));
      });
    });
    assert.equal(new URL(this.base).hostname, "127.0.0.1");
    assert.ok((await stat(join(this.directory, ".data/rustsurvey.db"))).isFile());
  }

  async request(path, body, { method = "GET", participant = false } = {}) {
    if (this.#interrupted) throw new Error("Benchmark interrupted.");
    const headers = { "Content-Type": "application/json", "X-Requested-With": "RustSurvey" };
    if (participant) headers.Authorization = `Bearer ${"a".repeat(64)}`;
    else {
      headers.Cookie = this.cookie;
      headers["X-CSRF-Token"] = this.csrf;
    }
    return fetch(`${this.base}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(15000),
    });
  }

  async json(path, body, options, expected = 200) {
    const response = await this.request(path, body, options);
    assert.equal(response.status, expected, `${options?.method ?? "GET"} ${path}`);
    return response.json();
  }

  async login() {
    const response = await this.request(
      "/api/admin/login",
      { username: "admin", password: "synthetic-benchmark-password" },
      { method: "POST" },
    );
    assert.equal(response.status, 200);
    this.cookie = response.headers.getSetCookie()[0].split(";")[0];
    this.csrf = (await response.json()).csrfToken;
  }

  async stop() {
    const child = this.child;
    if (child === undefined || child.exitCode !== null || child.signalCode !== null) return;
    await new Promise((resolve) => {
      const timer = setTimeout(() => child.kill("SIGKILL"), 3000);
      child.once("exit", () => {
        clearTimeout(timer);
        resolve();
      });
      child.kill("SIGTERM");
    });
  }

  async dispose() {
    await this.stop();
    if (this.directory === undefined) return;
    const directory = await realpath(this.directory);
    assert.equal(dirname(directory).toLowerCase(), (await realpath(tmpdir())).toLowerCase());
    assert.ok(basename(directory).startsWith("rustsurvey-performance-"));
    await rm(directory, { recursive: true, force: true });
  }
}
