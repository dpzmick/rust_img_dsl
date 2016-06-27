pub struct InputBuilder { id: usize }

impl InputBuilder {
    pub fn build<E1: Expr + 'static, E2: Expr + 'static>(&self)
        -> fn(E1, E2) -> InputExpr
        {
            match self.id {
                0 => {
                    make_input!(input, 0);
                    input
                },
                1 => {
                    make_input!(input, 1);
                    input
                }
                _ => unimplemented!()
            }
        }
}

impl<E1: Expr + 'static, E2: Expr + 'static> Fn<(E1, E2)> for InputBuilder {
    extern "rust-call" fn call(&self, args: (E1, E2)) -> InputExpr {
        let (a1, a2) = args;
        self.build()(a1, a2)
    }
}

impl<E1: Expr + 'static, E2: Expr + 'static> FnMut<(E1, E2)> for InputBuilder {
    extern "rust-call" fn call_mut(&mut self, args: (E1, E2)) -> InputExpr {
        let (a1, a2) = args;
        self.build()(a1, a2)
    }
}

impl<E1: Expr + 'static, E2: Expr + 'static> FnOnce<(E1, E2)> for InputBuilder {
    type Output = InputExpr;
    extern "rust-call" fn call_once(self, args: (E1, E2)) -> InputExpr {
        let (a1, a2) = args;
        self.build()(a1, a2)
    }
}
