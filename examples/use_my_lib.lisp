; Demonstrates requiring another Lisp file as a module and using its functions.

(log/info "use_my_lib.lisp: Attempting to require my_math_lib.lisp...")

; Assuming my_math_lib.lisp is in the same directory or a known module path.
; The exact path might need adjustment based on your interpreter's module resolution.
; Using a relative path:
(let lib (require "./my_math_lib.lisp"))
; Or if your interpreter automatically adds .lisp and searches paths:
; (let lib (require "my_math_lib"))


(log/info "use_my_lib.lisp: my_math_lib required. Accessing functions...")

; Call functions from the loaded module.
; The functions are accessed as if they are members of the 'lib' module object.
(log/info "Square of 7 (from lib):" (lib/square 7))
(log/info "Double of 12 (from lib):" (lib/double 12))

(log/info "")
(log/info "Demonstrating direct math module usage for comparison (if math is global):")
(log/info "Global + of 3 and 4:" (+ 3 4))

(log/info "")
(log/info "Using string formatting with a library function result:")
(log/info (string/format "The square of 9 is %s, and double of 15 is %s."
                         (lib/square 9)
                         (lib/double 15)))

(log/info "use_my_lib.lisp: Finished.")
