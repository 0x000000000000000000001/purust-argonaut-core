import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const root = fileURLToPath(new URL("../", import.meta.url));

function normalized(value) {
  if (value === null) return ["null"];
  if (typeof value === "number") {
    const bytes = Buffer.alloc(8);
    bytes.writeDoubleBE(value);
    return ["number", bytes.toString("hex")];
  }
  if (typeof value === "string") return ["string", value];
  if (typeof value === "boolean") return ["boolean", value];
  if (Array.isArray(value)) return ["array", value.map(normalized)];
  return ["object", Object.fromEntries(Object.entries(value).map(([key, item]) => [key, normalized(item)]))];
}

function commonCases() {
  // The native parser intentionally preserves encoding/json's behavior on
  // isolated surrogates, invalid UTF-8 and float overflow. These are covered
  // by the Go differential tests, rather than treated as JS equivalences.
  const inputs = [
    "null", "true", "false", "0", "-0", "-0.0", "1e-10000", "-1e-10000",
    "9007199254740993", "1.7976931348623157e308", "5e-324", "2.2250738585072014e-308",
    '""', '"é漢💻"', '"\\ud83d\\udcbb"', '"\\u0000\\b\\f\\n\\r\\t\\\\\\/\\\""',
    "[]", "{}", '[null,true,false,-0,{},[],"a"]',
    '{"x":1,"x":2}', '{"x":1,"\\u0078":null}',
    '{"__proto__":{"safe":true},"constructor":[],"":null}',
    ' \t\n {"é":"雪","emoji":"💻"} \r\n',
  ];
  let state = 0x71c39ab5;
  const random = n => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return (state >>> 0) % n;
  };
  const strings = ["", "plain", "é雪💻", "line\nquote\"slash\\", "\u0000", "\u2028\u2029"];
  function value(depth) {
    switch (random(depth === 0 ? 4 : 6)) {
      case 0: return null;
      case 1: return random(2) === 1;
      case 2: return (random(200000) - 100000) / (1 + random(100));
      case 3: return strings[random(strings.length)];
      case 4: return Array.from({ length: random(6) }, () => value(depth - 1));
      default: return Object.fromEntries(Array.from({ length: random(6) }, (_, i) => [strings[random(strings.length)] + i, value(depth - 1)]));
    }
  }
  for (let i = 0; i < 300; i++) inputs.push(JSON.stringify(value(4)));
  return inputs.map(input => ({ input, expected: normalized(JSON.parse(input)) }));
}

test("native JSON parser differential checks and optional microbenchmarks", t => {
  const directory = mkdtempSync(join(tmpdir(), "gopurs-json-parser-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  copyFileSync(join(root, "src/Data/Argonaut/Parser.go"), join(directory, "Parser.go"));
  copyFileSync(join(root, "test/parser-native_test.go"), join(directory, "parser-native_test.go"));
  writeFileSync(join(directory, "go.mod"), "module argonaut-parser-test\n\ngo 1.22\n");
  writeFileSync(join(directory, "js-oracle.json"), JSON.stringify(commonCases()));
  const benchmark = process.env.ARGONAUT_PARSER_BENCH === "1";
  const race = process.env.ARGONAUT_PARSER_RACE === "1";
  assert.ok(!(benchmark && race), "run race checks separately from benchmarks");
  const args = benchmark
    ? ["test", "-run=^$", "-bench=" + (process.env.ARGONAUT_PARSER_BENCH_FILTER || "BenchmarkJSONParser"), "-benchmem", "-count=3", "-benchtime=200ms"]
    : ["test", "-count=1", "-v"];
  // Separate correctness/race runs from timing campaigns.
  if (race) args.push("-race");
  args.push("./...");
  const output = execFileSync("go", args, {
    cwd: directory, encoding: "utf8", timeout: 180000,
    env: { ...process.env, GOWORK: "off" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  assert.match(output, /^PASS$/m);
  t.diagnostic(output.trim());
});
