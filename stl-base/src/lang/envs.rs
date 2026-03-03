use std::rc::Rc;

use crate::lang::vals::ExpVal;

type Symbol = String;

pub trait Env<V: ExpVal>: std::fmt::Debug {
    fn get(&self, var: &str) -> Option<Rc<V>>;
    fn extend(&self, var: &str, val: V) -> Self;
}

impl<V: ExpVal> Env<V> for SimpleEnv<V> {
    fn get(&self, sym: &str) -> Option<Rc<V>> {
        match self {
            SimpleEnv::Empty => None,
            SimpleEnv::Extend { var, val, link } => {
                if sym == var {
                    Some(val.clone())
                } else {
                    link.get(sym)
                }
            }
        }
    }

    fn extend(&self, var: &str, val: V) -> Self {
        match self {
            SimpleEnv::Empty => Rc::new(SimpleEnv::Extend { var, val, link: })
            SimpleEnv::Extend { var, val, link } => todo!(),
        }
    }
}

#[derive(Clone, Debug)]
enum SimpleEnv<V: ExpVal> {
    Empty,
    Extend {
        var: Symbol,
        val: Rc<V>,
        link: Rc<SimpleEnv<V>>,
    },
}
