import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { rmSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { corePackages, createWorkspace, prepareFixture } from "../../gopurs/tools/test-workspace.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const compiler = resolve(root, "../gopurs");
const workspace = createWorkspace();
const env = { ...process.env, GOWORK: "off", GOMAXPROCS: "4" };
env.PATH = join(compiler, "node_modules/.bin") + ":" + env.PATH;
const run = (name, command, args, cwd) => {
  const result = spawnSync(command, args, { cwd, env, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 });
  const output = (result.stdout ?? "") + (result.stderr ?? "");
  const log = join(workspace, `${name}.log`);
  writeFileSync(log, output);
  if (result.error || result.status !== 0) {
    console.error(output.slice(-10000));
    throw result.error ?? new Error(`${name} failed (${result.status}); log: ${log}`);
  }
  console.log(`${name}: passed`);
};
let success = false;
try {
  const fixture = prepareFixture(compiler, workspace, join(root, "test/compact-dom.purs"), 0, corePackages(compiler));
  run("purescript", "spago", ["build", "-q"], fixture.directory);
  run("javascript", "node", ["--input-type=module", "-e", "import { main } from './output/Main/index.js'; main();"], fixture.directory);
  run("generate-go", join(compiler, "bin/gopurs-native"), ["--main", "Main"], fixture.directory);
  const output = join(fixture.directory, "output");
  run("go-dependencies", "go", ["mod", "tidy"], output);
  run("go-race", "go", ["run", "-race", "./main"], output);
  success = true;
} finally {
  if (success) rmSync(workspace, { recursive: true, force: true });
  else console.error(`Kept failing compact-DOM workspace: ${workspace}`);
}
