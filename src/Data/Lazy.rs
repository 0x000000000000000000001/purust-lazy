use std::rc::Rc;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::thread::{self, ThreadId};

type PurustLazyCallback = purust_core::Func1<(), crate::UnknownType>;

enum PurustLazyState {
    Pending(PurustLazyCallback),
    Evaluating(ThreadId),
    Ready(crate::UnknownType),
}

// This is the foreign Data.Lazy.Lazy payload, not the Control.Lazy dictionary.
// Clone its Rc/Arc handle to share the computation and its memoized result.
pub struct Lazy {
    state: Mutex<PurustLazyState>,
    changed: Condvar,
}

impl Lazy {
    fn lock(&self) -> MutexGuard<'_, PurustLazyState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

// Keep the initializer available if evaluation unwinds. A panic is not a
// cached result: the next reader may retry, just as in the JavaScript FFI.
struct PurustLazyAttempt<'a> {
    lazy: &'a Lazy,
    callback: Option<PurustLazyCallback>,
}

impl Drop for PurustLazyAttempt<'_> {
    fn drop(&mut self) {
        if let Some(callback) = self.callback.take() {
            *self.lazy.lock() = PurustLazyState::Pending(callback);
            self.lazy.changed.notify_all();
        }
    }
}

pub fn Data_Lazy_defer(callback: PurustLazyCallback) -> Rc<Lazy> {
    Rc::new(Lazy {
        state: Mutex::new(PurustLazyState::Pending(callback)),
        changed: Condvar::new(),
    })
}

pub fn Data_Lazy_force(lazy: Rc<Lazy>) -> crate::UnknownType {
    let owner = thread::current().id();
    let mut state = lazy.lock();
    loop {
        match &*state {
            PurustLazyState::Ready(value) => return value.clone(),
            PurustLazyState::Evaluating(evaluator) if *evaluator == owner => {
                // Release the lock before unwinding so the outer attempt can
                // restore the initializer, or the callback can catch the panic.
                drop(state);
                panic!("Data.Lazy.force: reentrant evaluation");
            }
            PurustLazyState::Evaluating(_) => {
                state = lazy
                    .changed
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            PurustLazyState::Pending(_) => {
                let PurustLazyState::Pending(callback) =
                    std::mem::replace(&mut *state, PurustLazyState::Evaluating(owner))
                else {
                    unreachable!()
                };
                drop(state);
                let mut attempt = PurustLazyAttempt {
                    lazy: &lazy,
                    callback: Some(callback),
                };
                // No user callback or captured-value destructor runs under the
                // state lock. Different lazy cells may be forced here.
                let value = (attempt.callback.as_ref().unwrap())(());
                *lazy.lock() = PurustLazyState::Ready(value.clone());
                let callback = attempt.callback.take();
                lazy.changed.notify_all();
                drop(callback);
                return value;
            }
        }
    }
}
