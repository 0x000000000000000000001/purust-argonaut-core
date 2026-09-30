module Main where

-- @dependencies: assert prelude effect console either maybe argonaut-core foreign-object tuples arrays integers foldable-traversable partial st
import Prelude

import Control.Monad.ST as ST
import Data.Argonaut.Core (Json, fromArray, fromNumber, fromObject, fromString, jsonNull, stringify, stringifyWithIndent, toObject)
import Data.Argonaut.Parser (jsonParser)
import Data.Array as Array
import Data.Either (Either(..))
import Data.Foldable (for_)
import Data.Int as Int
import Data.Maybe (Maybe(..))
import Data.Tuple (Tuple(..))
import Effect (Effect)
import Effect.Console (log)
import Foreign.Object as Object
import Foreign.Object.ST as OST
import Partial.Unsafe (unsafeCrashWith)
import Test.Assert (assert, assertEqual)

parse :: String -> Json
parse text = case jsonParser text of
  Left err -> unsafeCrashWith err
  Right value -> value

main :: Effect Unit
main = do
  -- Exercise both sides of the compact cutoff through the public JSON and
  -- Foreign.Object APIs. Nested objects must survive boxing and map copies.
  for_ (Array.range 0 12) \size -> do
    let entries = map (\i -> Tuple ("k" <> show i)
          (fromArray [ jsonNull, fromNumber (Int.toNumber i), fromObject (Object.singleton "text" (fromString "kept")) ]))
          (Array.take size (Array.range 0 12))
    let expected = Object.fromFoldable entries
    let input = parse (stringify (fromObject expected))
    case toObject input of
      Nothing -> unsafeCrashWith "parsed object lost its shape"
      Just fields -> do
        let before = stringify input
        assert $ fields == expected
        assert $ compare input (fromObject expected) == EQ
        assert $ parse (stringifyWithIndent 2 input) == input
        assertEqual { expected: size, actual: Object.size fields }
        assertEqual { expected: size, actual: Object.fold (\n _ _ -> n + 1) 0 fields }
        assertEqual { expected: Array.sort (Object.keys expected), actual: Array.sort (Object.keys fields) }
        for_ entries \(Tuple key value) -> assert $ Object.lookup key fields == Just value
        assert $ Object.lookup "absent" fields == Nothing
        assert $ Object.mapWithKey (\_ value -> value) fields == expected
        assert $ Object.filterKeys (const true) fields == expected
        let inserted = Object.insert "new" jsonNull fields
        assert $ Object.lookup "new" inserted == Just jsonNull
        assert $ Object.delete "new" inserted == expected
        assert $ Object.union fields (Object.singleton "new" jsonNull) == inserted
        let Tuple frozen later = ST.run do
              mutable <- Object.thawST fields
              _ <- OST.poke "new" jsonNull mutable
              frozen <- Object.freezeST mutable
              _ <- OST.delete "new" mutable
              later <- Object.freezeST mutable
              pure (Tuple frozen later)
        assert $ frozen == inserted
        assert $ later == expected
        assertEqual { expected: before, actual: stringify input }
  log "compact DOM interoperability: Done"
