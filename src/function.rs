use expression::*;

#[derive(Debug)]
pub struct Function<'a> {
    num_inputs: usize,
    e:          Box<Expr + 'a>
}

impl<'a> Function<'a> {
    // wow
    pub fn new<F>(num_inputs: usize, gen: F) -> Self
        where F: Fn(
                  &Fn() -> Box<Expr + 'a>,
                  &Fn() -> Box<Expr + 'a>,
                  Vec<Box<Fn(Box<Expr + 'a>, Box<Expr + 'a>) -> Box<Expr + 'a>>>)
                  -> Box<Expr + 'a>
    {
        // create two functions which can be used to get a new boxed VarRef
        // we need this because all of the operators consume the boxes the operate on and its not a
        // good idea to try to implement Copy for a generic Box<Expr>, so each usage of a reference
        // to X or Y must exist inside of its own box
        let x = | | Box::new(VarRef::new(Var::X)) as Box<Expr + 'a>;
        let y = | | Box::new(VarRef::new(Var::Y)) as Box<Expr + 'a>;

        let mut vec = Vec::new();
        for i in 0..num_inputs {
            let input: Box<Fn(Box<Expr + 'a>, Box<Expr + 'a>) -> Box<Expr + 'a>>
                = Box::new(move |x, y| (Box::new(InputExpr::new(i, x, y)) as Box<Expr>));
            vec.push(input);
        }

        let e = gen(&x, &y, vec);

        Function { e: e, num_inputs: num_inputs }
    }

    pub fn gen_3x3_kernel(k: [[i64; 3]; 3]) -> Self {
        Function::new(1, |x, y, inputs| {
            let input = &inputs[0];

              (input(x() - 1, y() - 1) * k[0][0])
            + (input(x() - 1, y()    ) * k[1][0])
            + (input(x() - 1, y() + 1) * k[2][0])
            + (input(x()    , y() - 1) * k[0][1])
            + (input(x()    , y()    ) * k[1][1])
            + (input(x()    , y() + 1) * k[2][1])
            + (input(x() + 1, y() - 1) * k[0][2])
            + (input(x() + 1, y()    ) * k[1][2])
            + (input(x() + 1, y() + 1) * k[2][2])
        })
    }

    pub fn get_expr(&'a self) -> &'a Box<Expr +'a> {
        &self.e
    }
}
