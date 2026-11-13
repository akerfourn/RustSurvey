import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import config from "../vite.config.ts";

test("test_vite_should_refuse_private_data_even_when_the_workspace_is_allowed", async () => {
  const project = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
  const data = join(project, ".data");
  await mkdir(data, { recursive: true, mode: 0o700 });
  const directory = await mkdtemp(join(data, "http-deny-test-"));
  const payload = "SYNTHETIC_PRIVATE_DATA_MUST_NOT_BE_SERVED";
  const path = join(directory, "private.json");
  await writeFile(path, payload, { mode: 0o600 });
  let server;
  try {
    server = await createServer({
      ...config,
      configFile: false,
      root: join(project, "web"),
      logLevel: "silent",
      server: {
        ...config.server,
        port: 0,
        open: false,
        hmr: false,
        fs: { ...config.server.fs, allow: [project] },
      },
    });
    await server.listen();
    const address = server.httpServer.address();
    assert.ok(address !== null && typeof address !== "string");
    const base = `http://127.0.0.1:${address.port}`;
    assert.equal((await fetch(base, { signal: AbortSignal.timeout(5000) })).status, 200);
    for (const privatePath of [path, path.replace(".data", "%2edata")]) {
      const response = await fetch(`${base}/@fs/${privatePath}`, {
        signal: AbortSignal.timeout(5000),
      });
      assert.equal(response.status, 403);
      assert.ok(!(await response.text()).includes(payload));
    }
  } finally {
    if (server !== undefined) await server.close();
    await rm(directory, { recursive: true, force: true });
  }
});
