; A simple library of math utility functions.

(log/info "my_math_lib.lisp: Defining square and double functions...")

(let square (fn (x) (* x x)))
(let double (fn (x) (* x 2)))

(log/info "my_math_lib.lisp: square and double functions defined.")

; To make these functions available when this file is required as a module,
; they are now part of this file's environment.
; The 'require' special form will make this environment accessible.
