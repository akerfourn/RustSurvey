import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { setTimeout } from "node:timers/promises";

const engines = {
  postgres: {
    image: "postgres:17-alpine",
    port: "5432/tcp",
    scheme: "postgres",
    directory: "/var/lib/postgresql/data",
    variables: [
      "POSTGRES_USER=rustsurvey",
      "POSTGRES_PASSWORD=synthetic-test-password",
      "POSTGRES_DB=rustsurvey_test",
    ],
    ready: ["pg_isready", "-U", "rustsurvey", "-d", "rustsurvey_test"],
  },
  mariadb: {
    image: "mariadb:11.4",
    port: "3306/tcp",
    scheme: "mysql",
    directory: "/var/lib/mysql",
    variables: [
      "MARIADB_USER=rustsurvey",
      "MARIADB_PASSWORD=synthetic-test-password",
      "MARIADB_DATABASE=rustsurvey_test",
      "MARIADB_ROOT_PASSWORD=synthetic-root-password",
    ],
    ready: ["healthcheck.sh", "--connect", "--innodb_initialized"],
  },
};
const selected = process.argv[2] ?? "all";
if (process.argv.length > 3 || (selected !== "all" && !Object.hasOwn(engines, selected))) {
  console.error("Usage: npm run test:databases -- [all|postgres|mariadb]");
  process.exit(1);
}

let interrupted = false;
let activeChild = null;
process.on("SIGINT", () => {
  interrupted = true;
  activeChild?.kill("SIGINT");
});
process.on("SIGTERM", () => {
  interrupted = true;
  activeChild?.kill("SIGTERM");
});

function run(command, args, { visible = false, env = process.env, cleanup = false } = {}) {
  if (interrupted && !cleanup) return Promise.reject(new Error("Tests interrompus."));
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      env,
      stdio: visible ? "inherit" : ["ignore", "pipe", "pipe"],
    });
    activeChild = child;
    let stdout = "";
    if (!visible)
      child.stdout.on("data", (chunk) => {
        stdout += chunk.toString();
      });
    if (!visible) child.stderr.on("data", () => {});
    child.on("error", reject);
    child.on("close", (code) => {
      if (activeChild === child) activeChild = null;
      resolve({ code, stdout: stdout.trim() });
    });
  });
}

async function test(name, engine) {
  const image = await run("docker", ["image", "inspect", engine.image]);
  if (image.code !== 0)
    throw new Error(`Image absente. Charger explicitement : docker pull ${engine.image}`);
  const container = `rustsurvey-test-${name}-${randomUUID()}`;
  console.log(`Tests ${name} : conteneur jetable, données en mémoire, accès local uniquement.`);
  try {
    const variables = engine.variables.flatMap((value) => ["--env", value]);
    const started = await run("docker", [
      "run",
      "--detach",
      "--rm",
      "--pull=never",
      "--memory=512m",
      "--cpus=2",
      "--pids-limit=128",
      "--name",
      container,
      "--tmpfs",
      `${engine.directory}:rw,nosuid,noexec,size=384m`,
      "--publish",
      `127.0.0.1::${engine.port}`,
      ...variables,
      engine.image,
    ]);
    if (started.code !== 0) throw new Error(`Impossible de démarrer ${name}.`);
    const deadline = Date.now() + 60_000;
    let ready = false;
    while (Date.now() < deadline) {
      const probe = await run("docker", ["exec", container, ...engine.ready]);
      if (probe.code === 0) {
        ready = true;
        break;
      }
      await setTimeout(1000);
    }
    if (!ready) throw new Error(`${name} indisponible après 60 secondes.`);
    const published = await run("docker", ["port", container, engine.port]);
    if (published.code !== 0 || !/^127\.0\.0\.1:\d+$/.test(published.stdout))
      throw new Error("Port local inattendu.");
    const url = `${engine.scheme}://rustsurvey:synthetic-test-password@${published.stdout}/rustsurvey_test`;
    const result = await run(
      "cargo",
      [
        "test",
        "-p",
        "rustsurvey-server",
        "--all-features",
        "--locked",
        "--test",
        "workspace",
        "test_external_database_should_apply_migrations_and_roundtrip_large_documents",
        "--",
        "--ignored",
        "--exact",
      ],
      { visible: true, env: { ...process.env, RUSTSURVEY_TEST_DATABASE_URL: url } },
    );
    if (result.code !== 0) throw new Error(`Échec des tests ${name}.`);
    console.log(`Tests ${name} réussis.`);
  } finally {
    const removed = await run("docker", ["rm", "--force", container], { cleanup: true });
    if (removed.code !== 0) console.error(`Vérifier le nettoyage du conteneur ${container}.`);
  }
}

try {
  for (const [name, engine] of Object.entries(engines)) {
    if (selected === "all" || selected === name) await test(name, engine);
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : "Échec des tests SQL.");
  process.exitCode = 1;
}
