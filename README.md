# rsp - Rust S-expression Processor

`rsp` is an experimental Lisp-like interpreter written in Rust. This project serves as a test bed for exploring the capabilities of AI-assisted development, specifically with Gemini 2.5, in building a programming language interpreter.

## Features

*   Basic arithmetic operations (`+`, `*`, `=`)
*   Variable binding with `let`
*   First-class functions with lexical closures using `fn`
*   Conditional evaluation with `if`
*   Quoting with `quote`
*   String and number literals
*   Boolean literals (`true`, `false`) and `nil`
*   Module system:
    *   Loading files with `(require "path/to/file")` or `(require path/to/file-symbol)`.
    *   Built-in `math` module (e.g., `(math/+ 1 2)`)
    *   Built-in `log` module with `(log/info ...)` and `(log/error ...)` functions.
*   Basic `%s` string interpolation in `log/info` and `log/error`.

## Building

To build the interpreter, you need Rust installed. Then, navigate to the project directory and run:

```bash
cargo build
```

## Usage

The interpreter can be run using `cargo run --` followed by its arguments. The primary command is `run`.

### Evaluating Expressions from a String

You can evaluate a single Lisp expression provided as a string:

```bash
cargo run -- run --expr "(+ 10 20)"
```
Output:
```
Number(30.0)
```

```bash
cargo run -- run --expr "(let x 5) (* x (+ x 3)))"
```
Output:
```
Number(40.0)
```

### Evaluating Expressions from a File

You can execute a Lisp file. The interpreter will evaluate all expressions in the file and return an `Expr::Module` representing the file's environment.

Create a file, e.g., `examples/my_program.lisp`:
```lisp
(let initial-value 10)
(let my-adder (fn (a b) (+ a b initial-value)))
(my-adder 5 3) ; This will be the last evaluated expression if not for module return
```

Then run:
```bash
cargo run -- run examples/my_program.lisp
```
Output (path will vary):
```
Module(LispModule { path: "examples/my_program.lisp", env: "<module_env>" })
```

### Using Modules

Create `examples/my_lib.lisp`:
```lisp
(let lib-var 100)
(let lib-func (fn (x) (+ x lib-var)))
```

Then, in another expression or file:
```bash
cargo run -- run --expr "(require \"examples/my_lib\")"
```
Output (path will vary):
```
Module(LispModule { path: "/path/to/your/project/examples/my_lib.lisp", env: "<module_env>" })
```

You can then use functions from the loaded module (once member access is fully implemented beyond the current `module/member` symbol convention for built-ins).

### Using Built-in Modules

**Math Module:**
```bash
cargo run -- run --expr "(math/+ 10 (math/* 2 3))"
```
Output:
```
Number(16.0)
```

**Log Module:**
```bash
cargo run -- run --expr "(log/info \"Hello, %s. Your value is %s.\" \"User\" (+ 10 5))"
```
Output to stdout:
```
Hello, User. Your value is 15.
```
Return value:
```
String("Hello, User. Your value is 15.")
```

```bash
cargo run -- run --expr "(log/error \"An error occurred: %s\" \"File not found\")"
```
Output to stderr:
```
An error occurred: File not found
```
Return value:
```
String("An error occurred: File not found")
```

## Development Notes

This project is an ongoing experiment. The primary goal is to explore AI-assisted development in the context of interpreter design. Features and architectural decisions are influenced by this experimental nature.

## Future Directions (Potential)

*   More robust error handling with line/column numbers.
*   Advanced module member access syntax (e.g., `(module-ref math '+)`).
*   Macros.
*   Garbage collection (if heap-allocated Lisp objects are introduced beyond current `Rc`).
*   Expanded standard library.
*   Tail call optimization.
```
