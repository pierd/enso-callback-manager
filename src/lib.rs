#![feature(fn_traits)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]

use std::{marker::Tuple, rc};

#[derive(Clone)]
pub struct Handle(rc::Rc<()>);

struct Callback<Args> {
    alive: rc::Weak<()>,
    closure: Box<dyn FnMut<Args, Output = ()>>,
}

impl<Args: Tuple> Callback<Args> {
    fn is_alive(&self) -> bool {
        self.alive.upgrade().is_some()
    }

    fn call(&mut self, args: Args) {
        self.closure.call_mut(args);
    }
}

fn wrap<F, Args: Tuple>(callback: F) -> (Handle, Callback<Args>)
where
    F: FnMut<Args, Output = ()> + 'static,
{
    let handle = rc::Rc::new(());
    let alive = rc::Rc::downgrade(&handle);
    let closure = Box::new(callback);
    (Handle(handle), Callback { alive, closure })
}

#[derive(Default)]
pub struct CallbackManager<Args> {
    callbacks: Vec<Callback<Args>>,
}

impl<Args: Tuple> CallbackManager<Args> {
    pub fn add<F>(&mut self, callback: F) -> Handle
    where
        F: FnMut<Args, Output = ()> + 'static,
    {
        let (handle, callback) = wrap(callback);
        self.callbacks.push(callback);
        handle
    }

    pub fn run_all(&mut self, args: Args)
    where
        Args: Clone,
    {
        self.callbacks.retain(Callback::is_alive);
        for f in &mut self.callbacks {
            f.call(args.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[test]
    fn test_callbacks() {
        let mut manager = CallbackManager::default();

        // slice to check side-effects
        let counts = rc::Rc::new(RefCell::new([0; 3]));

        // create callback for each count entry
        let _handles: Vec<Handle> = counts
            .borrow()
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                let counts = rc::Rc::clone(&counts);
                manager.add(move |n: usize| RefCell::borrow_mut(&counts)[idx] += n)
            })
            .collect();

        manager.run_all((42,));
        assert_eq!(counts.borrow().as_slice(), &[42, 42, 42]);
    }

    #[test]
    fn test_handle_dropping() {
        let mut manager = CallbackManager::default();

        // slice to check side-effects
        let counts = rc::Rc::new(RefCell::new([0; 3]));

        // create callback for each count entry
        let mut handles: Vec<Handle> = counts
            .borrow()
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                let counts = rc::Rc::clone(&counts);
                manager.add(move |n: usize| RefCell::borrow_mut(&counts)[idx] += n)
            })
            .collect();

        // remove last callback
        handles.pop();

        manager.run_all((42,));
        assert_eq!(counts.borrow().as_slice(), &[42, 42, 0]);
    }
}
