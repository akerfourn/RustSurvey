import { Agent, request } from "node:http";

/** One independent HTTP/1.1 keep-alive connection per virtual browser. */
export class VirtualUser {
  #agent = new Agent({ keepAlive: true, maxSockets: 1, maxFreeSockets: 1, timeout: 120000 });
  #base;
  #token;
  id;
  version = 0;
  answers;
  status = "draft";
  entered = false;
  pending = null;

  constructor(base, id, token, answers) {
    this.#base = base;
    this.#token = token;
    this.id = id;
    this.answers = { ...answers };
  }

  get connections() {
    return [...Object.values(this.#agent.sockets), ...Object.values(this.#agent.freeSockets)]
      .flat()
      .filter((socket) => !socket.destroyed).length;
  }

  close() {
    this.#agent.destroy();
  }

  async request(
    path,
    { method = "GET", body, authorized = false, signal, limit = 8 * 1024 * 1024 } = {},
  ) {
    const headers = { "X-Requested-With": "RustSurvey" };
    let payload;
    if (body !== undefined) {
      payload = Buffer.from(JSON.stringify(body));
      headers["Content-Type"] = "application/json";
      headers["Content-Length"] = String(payload.length);
    }
    if (authorized) headers.Authorization = `Bearer ${this.#token}`;
    return new Promise((resolve, reject) => {
      const req = request(
        new URL(path, this.#base),
        { method, headers, agent: this.#agent, signal },
        (response) => {
          const parts = [];
          let bytes = 0;
          response.on("data", (chunk) => {
            bytes += chunk.length;
            if (bytes > limit) {
              response.destroy(new Error("Benchmark response exceeds its size limit."));
              return;
            }
            parts.push(chunk);
          });
          response.on("error", reject);
          response.on("end", () =>
            resolve({
              status: response.statusCode,
              headers: response.headers,
              bytes,
              body: Buffer.concat(parts),
              sentBytes: payload?.length ?? 0,
            }),
          );
        },
      );
      req.setTimeout(15000, () => req.destroy(new Error("Benchmark request timed out.")));
      req.on("error", reject);
      req.end(payload);
    });
  }
}
