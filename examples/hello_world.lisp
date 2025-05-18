; examples/my_program.lisp - (originally hello_world.lisp)
; A slightly more involved example program.

(log/info "Hello from rsp Lisp!")

(let name "User")
(log/info (string/format "Greetings, %s!" name))

(let a 15)
(let b 27)
(log/info (string/format "Calculation: (%s + %s) =" a b) (+ a b))

(if (> (+ a b) 40)
    (log/info "The sum is greater than 40.")
    (log/info "The sum is not greater than 40."))
