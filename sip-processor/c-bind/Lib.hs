{-# LANGUAGE ForeignFunctionInterface #-}

module Lib where

import Foreign.C.Types

foreign export ccall helloFromHaskell :: IO ()
helloFromHaskell = putStrLn "Hello World!"

foreign export ccall jomamaFromHaskell :: IO ()
jomamaFromHaskell = putStrLn "Jomama!"