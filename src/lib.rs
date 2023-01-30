#![feature(fn_traits)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]

use std::{cell::RefCell, marker::Tuple, rc::Rc};

#[derive(Clone)]
pub struct Handle(Rc<RefCell<bool>>);

impl Drop for Handle {
    fn drop(&mut self) {
        *RefCell::borrow_mut(&self.0) = false;
    }
}

struct Callback<Args> {
    alive: Rc<RefCell<bool>>,
    closure: Box<dyn FnMut<Args, Output = ()>>,
}

impl<Args: Tuple> Callback<Args> {
    fn wrap<F>(callback: F) -> Self
    where
        F: FnMut<Args, Output = ()> + 'static,
    {
        let alive = Rc::new(RefCell::new(true));
        let closure = Box::new(callback);
        Self { alive, closure }
    }

    fn get_handle(&self) -> Handle {
        Handle(self.alive.clone())
    }

    fn is_alive(&self) -> bool {
        *self.alive.borrow()
    }

    fn call(&mut self, args: Args) {
        self.closure.call_mut(args);
    }
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
        let callback = Callback::wrap(callback);
        let handle = callback.get_handle();
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
    use super::*;

    #[test]
    fn test_callbacks() {
        let mut manager = CallbackManager::default();

        // slice to check side-effects
        let counts = Rc::new(RefCell::new([0; 3]));

        // create callback for each count entry
        let _handles: Vec<Handle> = counts
            .borrow()
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                let counts = Rc::clone(&counts);
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
        let counts = Rc::new(RefCell::new([0; 3]));

        // create callback for each count entry
        let mut handles: Vec<Handle> = counts
            .borrow()
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                let counts = Rc::clone(&counts);
                manager.add(move |n: usize| RefCell::borrow_mut(&counts)[idx] += n)
            })
            .collect();

        // remove last callback
        handles.pop();

        manager.run_all((42,));
        assert_eq!(counts.borrow().as_slice(), &[42, 42, 0]);
    }
}
