; Demonstrates requiring another Lisp file as a module and using its functions.

(log/info "use_my_lib.lisp: Attempting to require my_lib.lisp...")

; Using a path relative to the project root (where cargo run is executed):
(let lib (require 'examples/my_lib))
; The 'require' form will use the symbol's name "examples/my_lib"
; and the module loader should append ".lisp" to find "examples/my_lib.lisp".


(log/info "use_my_lib.lisp: my_lib required. Accessing functions and variables...")

; Call functions and access variables from the loaded module.
(log/info (lib/greet "User"))
(log/info "Value of pi from lib:" (lib/pi))

(log/info "")
(log/info "Demonstrating direct math module usage for comparison (if math is global):")
(log/info "Global + of 3 and 4:" (+ 3 4))

(log/info "")
(log/info "Using string formatting with a library function result and variable:")
(log/info (string/format "Greeting: %s. Pi times two is %s."
                         (lib/greet "Another User")
                         (* (lib/pi) 2)))

(log/info "use_my_lib.lisp: Finished.")
