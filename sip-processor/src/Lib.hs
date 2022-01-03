{-# LANGUAGE ForeignFunctionInterface #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE FlexibleInstances #-}
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE TypeFamilies #-}
{-# LANGUAGE UndecidableInstances #-}

module Lib where

import Foreign.C.Types
import Foreign.C.String

foreign export ccall helloFromHaskell :: IO ()
helloFromHaskell = putStrLn "Hello World!"

foreign export ccall jomamaFromHaskell :: IO ()
jomamaFromHaskell = putStrLn "Jomama!"

isHelloWorldHs :: String -> Bool 
isHelloWorldHs "Hello World!" = True
isHelloWorldHs _ = False

foreign export ccall isHelloWorld :: CString -> IO CInt
isHelloWorld cStr = do
    str <- peekCString cStr
    return (if isHelloWorldHs str then 1 else 0)
