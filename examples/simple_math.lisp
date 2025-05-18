; Demonstrates basic arithmetic operations and variable definitions.

(log/info "Simple Arithmetic:")
(log/info "1 + 2 =" (+ 1 2))
(log/info "10 - 3 =" (- 10 3))
(log/info "4 * 5 =" (* 4 5))
(log/info "20 / 4 =" (/ 20 4))
(log/info "1 / 2 =" (/ 1 2)) ; Floating point division

(log/info "") ; Empty line for spacing

(log/info "Using Variables:")
(define x 10)
(define y 5)

(log/info "x =" x)
(log/info "y =" y)
(log/info "x + y =" (+ x y))
(log/info "x * y =" (* x y))

(define z (+ (* x 2) y))
(log/info "z = (x * 2) + y =" z)

(log/info "")
(log/info "Boolean operations:")
(log/info "(= 5 5) ->" (= 5 5))
(log/info "(= 5 3) ->" (= 5 3))
(log/info "(< 2 5) ->" (< 2 5))
(log/info "(> 2 5) ->" (> 2 5))
