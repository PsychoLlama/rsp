# rsp - Rust S-expression Processor

`rsp` is a Lisp-like interpreter written in Rust, 100% vibe coded by Gemini 2.5. It's a journey into AI-assisted programming language creation.

## Features

*   Arithmetic: `+`, `*`, `=`
*   Variables: `(let x 10)`
*   Functions: `(fn (a b) (+ a b))` with lexical closures
*   Conditionals: `(if condition then-expr else-expr)`
*   Quoting: `(quote foo)` or `'foo`
*   Literals: Numbers, Strings (`"hello"`), Booleans (`true`, `false`), `nil`
*   Modules:
    *   Load files: `(require "path/to/file")`
    *   Built-in `math` module: `(math/+ 1 2)`
    *   Built-in `log` module: `(log/info "Msg: %s" val)`, `(log/error "Err: %s" val)`
        *   Supports `%s` interpolation.
        *   `log/info` and `log/error` return the formatted string.

## Building

```bash
cargo build
```

## Usage

Run expressions directly:
```bash
cargo run -- run --expr "(math/+ 10 (math/* 2 3))"
```
Output:
```
Number(16.0)
```

Run a Lisp file:
```bash
cargo run -- run examples/my_program.lisp
```
Output (path will vary):
```
Module(LispModule { path: "examples/my_program.lisp", env: "<module_env>" })
```

Load a module:
```bash
cargo run -- run --expr "(require \"examples/my_lib\")"
```

Log messages:
```bash
cargo run -- run --expr "(log/info \"Value: %s\" (+ 10 5))"
```
Stdout: `Value: 15`
Returns: `String("Value: 15")`
```
