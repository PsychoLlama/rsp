use crate::ast::Expr;
use crate::env::Environment;
use crate::eval::LispError; // eval_let needs to return LispError
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{debug, error, trace};

#[tracing::instrument(skip(args, env), fields(args = ?args), ret, err)]
pub fn eval_let(args: &[Expr], env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
    trace!("Executing 'let' special form");
    if args.len() != 2 {
        error!(
            "'let' special form requires 2 arguments (variable name and value), found {}",
            args.len()
        );
        return Err(LispError::ArityMismatch(format!(
            "'let' expects 2 arguments, got {}",
            args.len()
        )));
    }

    let var_name_expr = &args[0];
    let value_expr = &args[1];

    let var_name = match var_name_expr {
        Expr::Symbol(name) => name.clone(),
        _ => {
            error!(
                "First argument to 'let' must be a symbol, found {:?}",
                var_name_expr
            );
            return Err(LispError::TypeError {
                expected: "Symbol".to_string(),
                found: format!("{:?}", var_name_expr),
            });
        }
    };

    debug!(variable_name = %var_name, value_expression = ?value_expr, "'let' binding");
    // Note: We need to call back into the main eval function here.
    // This requires `crate::eval::eval` to be accessible.
    let evaluated_value = crate::eval::eval(value_expr, Rc::clone(&env))?;

    env.borrow_mut()
        .define(var_name.clone(), evaluated_value.clone());
    debug!(variable_name = %var_name, value = ?evaluated_value, "Defined variable in environment using 'let'");
    Ok(evaluated_value)
}

// Future built-in functions will go here.

#[cfg(test)]
mod tests {
    // If we had tests specific to the internal logic of builtins, they would go here.
    // For now, `let` is tested via `eval.rs` which covers its integration.
}
