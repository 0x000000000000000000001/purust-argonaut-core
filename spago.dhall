{ name = "argonaut-core"
, dependencies =
  [ "arrays"
  , "console"
  , "control"
  , "effect"
  , "either"
  , "foreign"
  , "foreign-object"
  , "functions"
  , "gen"
  , "maybe"
  , "nonempty"
  , "partial"
  , "prelude"
  , "quickcheck"
  , "strings"
  , "tailrec"
  , "tuples"
  ]
, packages = ./packages.dhall
, sources = [ "src/**/*.purs", "test/**/*.purs" ]
}
