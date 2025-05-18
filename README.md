# rsp - A Lisp-like Interpreter in Rust

**This project is an experiment in AI-assisted development, not an attempt to create a production-ready Lisp interpreter.** Its primary goal is to push the capabilities of AI models, specifically Google's Gemini 1.5 Pro, by tasking them with a complex, iterative software engineering challenge: designing and implementing a programming language.

**100% of the code in this repository, including the language design, interpreter, and all features, was written by the AI** through interaction with [Aider](https://aider.chat/). This README file itself is also AI-generated. The project aims to explore the current state of AI-assisted development.

## Language Features

The `rsp` Lisp dialect supports a range of features:

*   **Literals**: Numbers (e.g., `123`, `-10.5`, `1.23e-4`), Strings (e.g., `"hello world"`), Booleans (`true`, `false`), and `nil`.
*   **Arithmetic & Comparison**:
    *   Basic arithmetic: `+`, `-`, `*`, `/`.
    *   Comparisons: `=`, `<`, `>`, `<=`, `>=`.
*   **Variables**: Define variables using `(let name value)`.
    *   Example: `(let x 10)`
*   **Functions**: First-class functions with lexical closures.
    *   Definition: `(fn (param1 param2) body-expr)`
    *   Example: `(let add (fn (a b) (+ a b)))`
*   **Conditionals**: `(if condition then-expr else-expr)`. The `else-expr` is optional; if omitted and the condition is false, `nil` is returned.
*   **Quoting**: Prevent evaluation using `(quote ...)` or the shorthand `'`.
    *   Example: `(quote foo)` or `'foo` results in the symbol `foo`.
    *   Example: `'(1 2 3)` results in the list `(1 2 3)`.
*   **Comments**: Lines starting with `;` are ignored.
    *   Example: `; this is a comment`
*   **Module System**:
    *   Load Lisp files as modules: `(require 'path/to/module)` (the `.lisp` extension is usually implicit). The path is typically relative to the interpreter's working directory.
    *   Access module members: `(module-name/function-name arg1 ...)` or `(module-name/variable-name)`.
    *   Example: If `my_lib.lisp` defines `(let my-val 42)`, you can use `(let m (require 'my_lib)) (m/my-val)`.
*   **Built-in Modules**:
    *   `log`: For printing messages.
        *   `(log/info arg1 arg2 ...)`: Prints arguments to standard output, space-separated.
        *   `(log/error arg1 arg2 ...)`: Prints arguments to standard error, space-separated.
        *   Both return the concatenated string of their arguments.
    *   `string`: For string operations.
        *   `(string/concat s1 s2 ...)`: Concatenates multiple strings.
        *   `(string/len s)`: Returns the length of string `s`.
        *   `(string/trim s)`: Trims leading/trailing whitespace from string `s`.
        *   `(string/to-upper s)`: Converts string `s` to uppercase.
        *   `(string/to-lower s)`: Converts string `s` to lowercase.
        *   `(string/reverse s)`: Reverses string `s`.
        *   `(string/format fmt-str arg1 ...)`: Formats a string using `%s` placeholders, similar to `printf`.
    *   `math`: Provides mathematical functions (in addition to the globally available arithmetic and comparison operators).
        *   Currently, the core math operators (`+`, `-`, `*`, `/`, `=`, `<`, `>`, `<=`, `>=`) are also available globally.

## Building

To build the interpreter:
```bash
cargo build
```

To build and run tests:
```bash
cargo test
```

## Usage

### Interactive REPL (Read-Eval-Print Loop)

Start the REPL:
```bash
cargo run -- repl
```
You can then type Lisp expressions directly. Use `.exit`, `(exit)`, Ctrl+D to quit, or Ctrl+C to interrupt.

### Running Expressions from the Command Line

Execute a single expression string:
```bash
cargo run -- run --expr "(+ 10 (* 2 3))"
```
Output:
```
Number(16.0)
```

### Running Lisp Files

Execute a Lisp file:
```bash
cargo run -- run examples/my_program.lisp
```
This will execute the expressions in the file. The output will be from `log/info` or `log/error` calls, and the final result of running a file is a `Module` expression representing the file's environment.

Example: `examples/my_program.lisp`
```lisp
; examples/my_program.lisp
(log/info "Hello from rsp Lisp!")
(log/info (string/format "Calculation: (%s + %s) =" 15 27) (+ 15 27))
```
Running `cargo run -- run examples/my_program.lisp` would output:
```
Hello from rsp Lisp!
Calculation: (15 + 27) = 42
Module(LispModule { path: "examples/my_program.lisp", env: "<module_env>" })
```

### Using Modules

1.  Create a library file, e.g., `examples/my_lib.lisp`:
    ```lisp
    ; examples/my_lib.lisp
    (log/info "my_lib.lisp: Loading library...")
    (let greet (fn (name) (string/format "Hello, %s, from my_lib!" name)))
    (let pi 3.14159)
    (log/info "my_lib.lisp: greet function and pi variable defined.")
    ```

2.  Create a main file, e.g., `examples/use_my_lib.lisp`:
    ```lisp
    ; examples/use_my_lib.lisp
    (let lib (require 'examples/my_lib))

    (log/info (lib/greet "User"))
    (log/info "Value of pi from lib:" (lib/pi))
    ```

3.  Run the main file:
    ```bash
    cargo run -- run examples/use_my_lib.lisp
    ```
    Output:
    ```
    my_lib.lisp: Loading library...
    my_lib.lisp: greet function and pi variable defined.
    Hello, User, from my_lib!
    Value of pi from lib: 3.14159
    Module(LispModule { path: "examples/use_my_lib.lisp", env: "<module_env>" })
    ```
