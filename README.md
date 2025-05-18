# rsp - A Lisp-like Interpreter in Rust

This project is an experiment with [Aider](https://aider.chat/) probing the capabilities of Gemini 2.5. Older models fail out and get stuck in loops when given high level tasks like "design a new language". Gemini 2.5 is the first model I've seen that can actually do it.

This is all entirely a proof of concept. I'm not using it for anything serious. It just explores the tools.

This project is 100% AI generated. I didn't write a single line of code manually.

## Features

- Arithmetic: `+`, `*`, `=`
- Variables: `(let x 10)`
- Functions: `(fn (a b) (+ a b))` with lexical closures
- Conditionals: `(if condition then-expr else-expr)`
- Quoting: `(quote foo)` or `'foo`
- Literals: Numbers, Strings (`"hello"`), Booleans (`true`, `false`), `nil`
- Modules:
  - Load files: `(require "path/to/file")`
  - Built-in `math` module: `(math/+ 1 2)`
  - Built-in `log` module: `(log/info "Msg: %s" val)`, `(log/error "Err: %s" val)`
    - Supports `%s` interpolation.
    - `log/info` and `log/error` return the formatted string.

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
