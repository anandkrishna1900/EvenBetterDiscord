;; Compile to hello.wasm before running the manifest:
;;   wat2wasm hello.wat -o hello.wasm
(module
  (import "env" "ebd_log" (func $log (param i32 i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "hello from a sandboxed plugin")
  (func (export "run") (result i32)
    i32.const 0 i32.const 29 call $log
    i32.const 0))
