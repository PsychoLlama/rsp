; Defines a recursive factorial function and calls it.

(define factorial
  (fn (n)
    (if (= n 0)
        1 ; Base case: 0! = 1
        (* n (factorial (- n 1)))))) ; Recursive step: n * (n-1)!

(log/info "Factorial of 0:" (factorial 0))
(log/info "Factorial of 1:" (factorial 1))
(log/info "Factorial of 5:" (factorial 5))
(log/info "Factorial of 7:" (factorial 7))

; Example of trying to calculate factorial of a larger number
; (log/info "Factorial of 10:" (factorial 10))
; Note: Deep recursion might be slow or hit limits depending on interpreter implementation.
