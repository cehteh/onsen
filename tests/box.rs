use onsen::*;

#[test]
fn smoke() {
    let pool = RcPool::new();
    let mybox = Box::new("Boxed", &pool);
    assert_eq!(*mybox, "Boxed");
}

#[test]
fn many_allocations() {
    let pool = RcPool::new();
    let mybox1 = Box::new("Boxed1", &pool);
    let mybox2 = Box::new("Boxed2", &pool);
    let mybox3 = Box::new("Boxed3", &pool);
    let mybox4 = Box::new("Boxed4", &pool);
    let mybox5 = Box::new("Boxed5", &pool);
    assert_eq!(*mybox1, "Boxed1");
    assert_eq!(*mybox2, "Boxed2");
    assert_eq!(*mybox3, "Boxed3");
    assert_eq!(*mybox4, "Boxed4");
    assert_eq!(*mybox5, "Boxed5");
}

#[test]
fn box_is_thin_pointer() {
    assert_eq!(
        std::mem::size_of::<Box<usize, RcPool<BoxInner<usize>>>>(),
        std::mem::size_of::<usize>()
    );
}

#[test]
fn frombox() {
    let pool = RcPool::new();
    let mybox = Box::new("Boxed1", &pool);
    let secondbox = Box::new("Boxed2", &mybox);
    assert_ne!(mybox, secondbox);
}

#[test]
#[cfg_attr(miri, ignore)] // miri would report leaked memory here
fn forget() {
    let pool = RcPool::new();
    Box::forget(Box::new("Boxed", &pool));
}

#[test]
fn into_inner() {
    let pool = RcPool::new();
    let v: &str = Box::into_inner(Box::new("Was Boxed", &pool));
    assert_eq!(v, "Was Boxed");
}

#[test]
fn deref() {
    let pool = RcPool::new();
    let mybox = Box::new("Boxed", &pool);
    assert_eq!(*mybox, "Boxed");
}

#[test]
fn deref_mut() {
    let pool = RcPool::new();
    let mut mybox = Box::new("Boxed", &pool);
    *mybox = "Changed";
    assert_eq!(*mybox, "Changed");
}

#[test]
fn eq() {
    let pool = RcPool::new();
    let box1 = Box::new("Boxed", &pool);
    let box2 = Box::new("Boxed", &pool);
    let box3 = Box::new("Boxed again", &pool);
    assert_eq!(box1, box2);
    assert_ne!(box1, box3);
}
