use std::rc::Rc;

use crate::lang::vals::ExpVal;

type Symbol = String;

enum SimpleEnv {
    Empty,
    Extend {
        var: Symbol,
        val: Rc<dyn ExpVal>,
        link: Rc<SimpleEnv>,
    },
}
