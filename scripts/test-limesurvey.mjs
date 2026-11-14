import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtemp, readFile, writeFile, mkdir, rm, realpath } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { setTimeout } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { cargoEnvironment } from "./cargo-environment.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const image = "rustsurvey-limesurvey-reference:6.5.14";
const token = randomUUID();
const network = `rustsurvey-reference-${token}`;
const database = `${network}-db`;
const application = `${network}-app`;
const temporary = await mkdtemp(join(tmpdir(), "rustsurvey-limesurvey-reference-"));
const environment = cargoEnvironment(process.env);
let interrupted = false;
let activeChild = null;
let networkCreated = false;
const containers = [];
function interrupt() {
  interrupted = true;
  activeChild?.kill("SIGINT");
}
process.on("SIGINT", interrupt);
process.on("SIGTERM", interrupt);

function run(command, args, { visible = false, cleanup = false, env = environment } = {}) {
  if (interrupted && !cleanup) return Promise.reject(new Error("Reference checks interrupted."));
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      env,
      cwd: root,
      timeout: 300_000,
      stdio: visible ? "inherit" : ["ignore", "pipe", "pipe"],
    });
    activeChild = child;
    let output = "";
    if (!visible) {
      child.stdout.on("data", (chunk) => {
        output = (output + chunk).slice(-16_384);
      });
      child.stderr.on("data", (chunk) => {
        output = (output + chunk).slice(-16_384);
      });
    }
    child.on("error", reject);
    child.on("close", (code) => {
      if (activeChild === child) activeChild = null;
      if (code !== 0)
        return reject(new Error(`${command} failed (${code}): ${output.slice(-4000)}`));
      resolve(output.trim());
    });
  });
}

async function prepare() {
  await run("docker", ["image", "inspect", image]);
  await run("docker", ["image", "inspect", "mariadb:11.4"]);
  const corpus = JSON.parse(
    await readFile(join(root, "examples/rules/collection-forms.json"), "utf8"),
  );
  const definition = corpus.definition;
  definition.languages = ["fr", "en"];
  definition.translations = [
    { language: "en", title: "Reference study", welcome: "Welcome", end: "Thank you" },
  ];
  for (const group of definition.groups) {
    group.translations = [{ language: "en", text: "Reference group", help: "Group description" }];
    for (const question of group.questions) {
      question.translations = [
        { language: "en", text: `${question.code} English`, help: "Question help" },
      ];
      for (const choice of [...question.choices, ...question.subquestions])
        choice.translations = [{ language: "en", text: `${choice.code} English`, help: null }];
    }
  }
  await writeFile(join(temporary, "definition.json"), JSON.stringify(definition));
  await mkdir(join(temporary, "output"));
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--test",
      "defaults_reference",
      "test_native_default_reference_should_prepare_versioned_structures",
      "--",
      "--ignored",
    ],
    {
      visible: true,
      env: { ...environment, RUSTSURVEY_DEFAULT_REFERENCE_DIR: temporary },
    },
  );
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--test",
      "filters_reference",
      "test_filter_reference_should_prepare_synthetic_structures",
      "--",
      "--ignored",
    ],
    {
      visible: true,
      env: { ...environment, RUSTSURVEY_FILTER_REFERENCE_DIR: temporary },
    },
  );
  await run(
    "cargo",
    [
      "run",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--",
      "export-native",
      join(temporary, "definition.json"),
      join(temporary, "input.lss"),
      "701",
    ],
    { visible: true },
  );
  await run("cargo", [
    "run",
    "-p",
    "rustsurvey-limesurvey",
    "--locked",
    "--",
    "export-bindings",
    join(temporary, "definition.json"),
    join(temporary, "bindings.json"),
    "701",
  ]);
  for (const format of ["group", "question", "all"]) {
    const settings = { format, showProgress: format === "group" };
    const settingsPath = join(temporary, `navigation-${format}.json`);
    await writeFile(settingsPath, JSON.stringify(settings));
    await run("cargo", [
      "run",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--",
      "export-native",
      join(temporary, "definition.json"),
      join(temporary, `navigation-${format}.lss`),
      "702",
      settingsPath,
    ]);
  }
  const bindings = JSON.parse(await readFile(join(temporary, "bindings.json"), "utf8"));
  const exclusiveDefinition = structuredClone(definition);
  for (const question of exclusiveDefinition.groups[0].questions) {
    if (!["A", "B", "C", "E", "F", "M", "P", "K", "Q"].includes(question.code)) continue;
    question.attributes.push({ name: "exclude_all_others", value: "A2", language: null });
    if (["M", "P"].includes(question.code))
      question.attributes.push({ name: "exclude_all_others_auto", value: "1", language: null });
  }
  await writeFile(
    join(temporary, "exclusive-definition.json"),
    JSON.stringify(exclusiveDefinition),
  );
  await run("cargo", [
    "run",
    "-p",
    "rustsurvey-limesurvey",
    "--locked",
    "--",
    "export-native",
    join(temporary, "exclusive-definition.json"),
    join(temporary, "exclusive.lss"),
    "703",
  ]);
  const columns = [
    "id",
    "submitdate",
    "lastpage",
    "startlanguage",
    "seed",
    ...bindings.map((binding) => binding.column),
  ];
  const first = {
    id: "1",
    submitdate: "2024-06-24 12:34:56",
    lastpage: null,
    startlanguage: "fr",
    seed: null,
  };
  const second = { id: "2", submitdate: null, lastpage: null, startlanguage: "en", seed: null };
  for (const binding of bindings) {
    first[binding.column] = corpus.baseline.expected[binding.runtimeKey] ?? null;
    second[binding.column] = null;
    if (binding.runtimeKey === "S") second[binding.column] = "";
    if (binding.runtimeKey === "T")
      second[binding.column] =
        '  Unicode \u00e9\u65e5\u672c & <tag> ]]>\r\n\t"text" O\'Neil `code` {question_not_shown} {lbrace}{quote}  ';
  }
  await writeFile(
    join(temporary, "responses.json"),
    JSON.stringify({ languages: ["fr", "en"], columns, rows: [first, second] }),
  );
  await run(
    "cargo",
    [
      "run",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--",
      "export-responses",
      join(temporary, "responses.json"),
      join(temporary, "responses.lsr"),
    ],
    { visible: true },
  );
  await writeFile(
    join(temporary, "aliases.json"),
    JSON.stringify(
      Object.fromEntries(bindings.map((binding) => [binding.column, binding.runtimeKey])),
    ),
  );
  await run(
    "cargo",
    [
      "run",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--",
      "export-vv",
      join(temporary, "responses.json"),
      join(temporary, "aliases.json"),
      join(temporary, "responses.vv"),
    ],
    { visible: true },
  );
  const campaignColumns = [
    "submitdate",
    "startlanguage",
    ...bindings.map((binding) => binding.runtimeKey).sort(),
  ];
  const campaignRows = [first, second].map((row) => {
    const result = { submitdate: row.submitdate, startlanguage: row.startlanguage };
    for (const binding of bindings) result[binding.runtimeKey] = row[binding.column];
    return result;
  });
  await writeFile(
    join(temporary, "campaign.json"),
    JSON.stringify({ languages: ["fr", "en"], columns: campaignColumns, rows: campaignRows }),
  );
  await writeFile(join(temporary, "campaign-aliases.json"), "{}");
  await run("cargo", [
    "run",
    "-p",
    "rustsurvey-limesurvey",
    "--locked",
    "--",
    "export-vv",
    join(temporary, "campaign.json"),
    join(temporary, "campaign-aliases.json"),
    join(temporary, "campaign.vv"),
  ]);
  return definition;
}

async function start() {
  await run("docker", ["network", "create", "--internal", network]);
  networkCreated = true;
  containers.push(database);
  await run("docker", [
    "run",
    "--detach",
    "--rm",
    "--pull=never",
    "--name",
    database,
    "--network",
    network,
    "--network-alias",
    "database",
    "--memory=512m",
    "--cpus=2",
    "--pids-limit=128",
    "--tmpfs",
    "/var/lib/mysql:rw,nosuid,noexec,size=384m",
    "--env",
    "MARIADB_DATABASE=rustsurvey_reference",
    "--env",
    "MARIADB_USER=reference",
    "--env",
    "MARIADB_PASSWORD=synthetic-reference-password",
    "--env",
    "MARIADB_ROOT_PASSWORD=synthetic-reference-root",
    "mariadb:11.4",
  ]);
  const deadline = Date.now() + 60_000;
  let ready = false;
  while (Date.now() < deadline) {
    try {
      await run("docker", [
        "exec",
        database,
        "healthcheck.sh",
        "--connect",
        "--innodb_initialized",
      ]);
      ready = true;
      break;
    } catch {
      if (interrupted) throw new Error("Reference checks interrupted.");
      await setTimeout(1000);
    }
  }
  assert.ok(ready, "disposable reference database must become ready");
  containers.push(application);
  await run("docker", [
    "run",
    "--detach",
    "--rm",
    "--pull=never",
    "--name",
    application,
    "--network",
    network,
    "--memory=768m",
    "--cpus=2",
    "--pids-limit=128",
    "--env",
    "RUSTSURVEY_REFERENCE_ONLY=1",
    "--mount",
    `type=bind,source=${temporary},target=/fixtures`,
    image,
    "sleep",
    "infinity",
  ]);
  await run(
    "docker",
    [
      "exec",
      application,
      "php",
      "application/commands/console.php",
      "install",
      "reference",
      "synthetic-reference-admin",
      "Synthetic Reference",
      "reference@example.invalid",
    ],
    { visible: true },
  );
}

try {
  const definition = await prepare();
  await writeFile(
    join(temporary, "default-cases.json"),
    await readFile(join(root, "examples/rules/defaults-reference.json")),
  );
  await start();
  for (const name of [
    "ReferenceCommand.php",
    "SettingsCases.php",
    "FilterCases.php",
    "DefaultCases.php",
    "ExecutionCases.php",
  ]) {
    await run("docker", [
      "cp",
      join(root, "tests/limesurvey-reference", name),
      `${application}:/opt/limesurvey/application/commands/${name}`,
    ]);
  }
  await run("docker", [
    "cp",
    join(root, "tools/limesurvey/RustSurveySettingsCommand.php"),
    `${application}:/opt/limesurvey/application/commands/RustSurveySettingsCommand.php`,
  ]);
  await run("docker", [
    "cp",
    join(root, "tools/limesurvey/RustSurveyRuntimeCommand.php"),
    `${application}:/opt/limesurvey/application/commands/RustSurveyRuntimeCommand.php`,
  ]);
  await run(
    "docker",
    [
      "exec",
      application,
      "php",
      "application/commands/console.php",
      "reference",
      "/fixtures/input.lss",
      "/fixtures/output",
    ],
    { visible: true },
  );
  const summary = JSON.parse(await readFile(join(temporary, "output/summary.json"), "utf8"));
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--test",
      "defaults_reference",
      "test_default_reference_",
      "--",
      "--ignored",
    ],
    {
      visible: true,
      env: { ...environment, RUSTSURVEY_DEFAULT_REFERENCE_DIR: temporary },
    },
  );
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--test",
      "filters_reference",
      "test_filter_reference_should_match_actual_limesurvey_row_relevance",
      "--",
      "--ignored",
    ],
    {
      visible: true,
      env: { ...environment, RUSTSURVEY_FILTER_REFERENCE_DIR: temporary },
    },
  );
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--test",
      "import_execution",
      "test_import_execution_reference_should_compile_an_actual_limesurvey_export",
      "--",
      "--ignored",
    ],
    {
      visible: true,
      env: { ...environment, RUSTSURVEY_EXECUTION_REFERENCE_DIR: temporary },
    },
  );
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-server",
      "--locked",
      "--test",
      "workspace",
      "test_import_execution_reference_should_collect_actual_source_after_http_import",
      "--",
      "--ignored",
    ],
    {
      visible: true,
      env: { ...environment, RUSTSURVEY_EXECUTION_REFERENCE_DIR: temporary },
    },
  );
  const expectedFacts = JSON.parse(
    await readFile(join(temporary, "output/execution-facts.json"), "utf8"),
  );
  const runtimeArguments = [
    "exec",
    application,
    "php",
    "application/commands/console.php",
    "rustsurveyruntime",
    expectedFacts.surveyId,
    "/fixtures/output/execution-facts-fresh.json",
  ];
  await run("docker", runtimeArguments);
  assert.deepEqual(
    JSON.parse(await readFile(join(temporary, "output/execution-facts-fresh.json"), "utf8")),
    expectedFacts,
  );
  await assert.rejects(run("docker", runtimeArguments), /failed/);
  const extracted = JSON.parse(
    await readFile(join(temporary, "output/settings-3-extracted.json"), "utf8"),
  );
  const helperArguments = [
    "exec",
    application,
    "php",
    "application/commands/console.php",
    "rustsurveysettings",
    extracted.surveyId,
    "/fixtures/output/cli-context.json",
  ];
  await run("docker", helperArguments);
  const cliContext = await readFile(join(temporary, "output/cli-context.json"));
  assert.deepEqual(JSON.parse(cliContext.toString("utf8")), extracted);
  await assert.rejects(run("docker", helperArguments), /failed/);
  assert.deepEqual(await readFile(join(temporary, "output/cli-context.json")), cliContext);
  assert.equal(summary.version, "6.5.14");
  assert.equal(Number(summary.schema), 623);
  assert.equal(Object.keys(summary.questionTypes).length, 28);
  assert.deepEqual([...summary.languages].sort(), ["en", "fr"]);
  for (const code of ["A", "B", "C", "E", "F", "M", "P", "K", "Q"]) {
    assert.equal(summary.exclusiveAttributes[code].exclude_all_others, "A2");
    if (["M", "P"].includes(code))
      assert.equal(summary.exclusiveAttributes[code].exclude_all_others_auto, "1");
  }
  for (const [format, code] of [
    ["group", "G"],
    ["question", "S"],
    ["all", "A"],
  ]) {
    assert.deepEqual(summary.navigation[format], {
      format: code,
      showprogress: format === "group" ? "Y" : "N",
      allowprev: "Y",
    });
  }
  await run(
    "cargo",
    ["test", "-p", "rustsurvey-limesurvey", "--locked", "--test", "reference", "--", "--ignored"],
    {
      visible: true,
      env: {
        ...environment,
        RUSTSURVEY_REFERENCE_DEFINITION: join(temporary, "definition.json"),
        RUSTSURVEY_REFERENCE_LSS: join(temporary, "output/roundtrip.lss"),
      },
    },
  );
  assert.equal(definition.groups[0].questions.length, 28);
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-limesurvey",
      "--locked",
      "--test",
      "settings_reference",
      "--",
      "--ignored",
    ],
    {
      visible: true,
      env: { ...environment, RUSTSURVEY_REFERENCE_LSS: join(temporary, "output/roundtrip.lss") },
    },
  );
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-server",
      "--locked",
      "--test",
      "workspace",
      "test_reference_import_should_preserve_actual_limesurvey_response_xml_and_cell_values",
      "--",
      "--ignored",
    ],
    {
      visible: true,
      env: { ...environment, RUSTSURVEY_REFERENCE_LSS: join(temporary, "output/roundtrip.lss") },
    },
  );
  console.log("Stored source settings: checking private, revision-bound contexts.");
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-server",
      "--locked",
      "--test",
      "workspace",
      "test_source_context_reference_should_persist_actual_limesurvey_options",
      "--",
      "--ignored",
    ],
    {
      visible: true,
      env: { ...environment, RUSTSURVEY_REFERENCE_LSS: join(temporary, "output/roundtrip.lss") },
    },
  );
  console.log(
    "LimeSurvey 6.5.14 / schema 623 : structures, cellules et réponses VV vérifiées sur deux études jetables.",
  );
} finally {
  for (const name of containers.reverse()) {
    try {
      await run("docker", ["rm", "--force", name], { cleanup: true });
    } catch {
      console.error(`Verify cleanup of ${name}.`);
    }
  }
  if (networkCreated) {
    try {
      await run("docker", ["network", "rm", network], { cleanup: true });
    } catch {
      console.error(`Verify cleanup of ${network}.`);
    }
  }
  const resolved = await realpath(temporary);
  assert.equal(dirname(resolved).toLowerCase(), (await realpath(tmpdir())).toLowerCase());
  assert.ok(basename(resolved).startsWith("rustsurvey-limesurvey-reference-"));
  await rm(resolved, { recursive: true, force: true });
}
