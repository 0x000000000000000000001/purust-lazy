use perceus_ptr::PerceusPtr;
type UnknownType = perceus_ptr::PerceusPtr<crate::Record_a>;

pub fn Data_Lazy_defer() -> UnknownType {
    PerceusPtr::new(crate::Record_a {
        call: Some(std::rc::Rc::new(move |mut thunk: UnknownType| -> UnknownType {
            thunk.clone()
        })),
        ..Default::default()
    })
}

pub fn Data_Lazy_force() -> UnknownType {
    PerceusPtr::new(crate::Record_a {
        call: Some(std::rc::Rc::new(move |mut lazy: UnknownType| -> UnknownType {
            lazy.call.clone().unwrap()(PerceusPtr::new(crate::Record_a { ..Default::default() }))
        })),
        ..Default::default()
    })
}
