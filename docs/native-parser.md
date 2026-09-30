# Native JSON parser

`Data.Argonaut.Parser.jsonParser` keeps its PureScript API. Its Go FFI builds
the existing `Json` tree directly while scanning the input string: `nil`,
`bool`, `float64`, `string`, `[]any` and `map[string]any`. It avoids converting
the whole document to bytes and the standard decoder's separate document
validation pass. The standard decoder already has specialized paths for `any`;
this change is not the removal of reflection from every JSON node.

The parser implements JSON whitespace, number grammar, escapes and collections.
`strconv.ParseFloat` provides the same number rounding, underflow and signed
zero as the standard decoder. Duplicate object keys keep the last value.
Empty arrays and objects remain non-nil and distinct from JSON null.

Strings own their storage: simple strings are cloned, and escaped strings use
a private builder. Keeping one small value cannot retain the whole input.
Invalid UTF-8 and isolated UTF-16 surrogates become U+FFFD, preserving the Go
behavior. This does not change the existing difference from JavaScript on lone
surrogates or overflowing numbers.

Malformed input, number overflow or excessive nesting falls back to
`encoding/json.Unmarshal` on a fresh destination. This preserves its exact
error messages and nesting limit. The failure path can perform extra work;
the optimization targets successful parsing. Exactly one supplied callback
runs, and callback panics propagate unchanged. All parser state is local to
the invocation.

## Validation

From this checkout:

```sh
node --test test/parser-native.test.mjs
ARGONAUT_PARSER_RACE=1 node --test test/parser-native.test.mjs
GOMAXPROCS=1 GOGC=100 ARGONAUT_PARSER_BENCH=1 node --test test/parser-native.test.mjs
```

The runner copies the actual FFI source into a temporary Go module. It compares
full values, float bits, exact errors and callback counts against the standard
decoder; it also compares a shared valid subset against `JSON.parse`.
Directed cases, deterministic mutations and truncations cover Unicode,
duplicates, malformed syntax, depth, concurrency and source-buffer retention.
Benchmarks run separately, without the race detector, across nine input shapes.

The application-level JSON and TAST results, including the unchanged JS controls,
are in the [altbak campaign](../../../altbak.pub-gopurs/docs/benchmark-results/2026-09-22-native-json-parser.md).
The PureScript decoders and the generated `Value` representation are unchanged.
