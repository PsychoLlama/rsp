; examples/my_lib.lisp - (originally my_math_lib.lisp)
; A simple library.

(log/info "my_lib.lisp: Loading library...")

(let greet (fn (name) (string/format "Hello, %s, from my_lib!" name)))
(let pi 3.14159)

(log/info "my_lib.lisp: greet function and pi variable defined.")
