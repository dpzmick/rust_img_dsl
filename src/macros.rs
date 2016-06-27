#[macro_export]
macro_rules! make_input(
    ($name:ident, $number:expr) => (
            fn $name<'a, E1: Expr + 'a, E2: Expr + 'a>(x: E1, y: E2) -> InputExpr<'a> {
                InputExpr::new($number, Box::new(x), Box::new(y))
            }
        );
    );
