module Test.Smoke where

import Prelude
import Effect (Effect)
import Effect.Console (log)
import Data.Argonaut.Core (Json, fromBoolean)

main :: Effect Unit
main = log "Hello"
