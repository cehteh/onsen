use onsen::*;

#[test]
fn smoke() {
    let pool = ArcPool::new();
    let _mybox = Box::new("Boxed", &pool);
}

#[test]
fn box_is_thin_pointer() {
    assert_eq!(
        std::mem::size_of::<Box<usize, ArcPool<FatPoolEntry<usize>>>>(),
        std::mem::size_of::<usize>()
    );
}

#[test]
fn frombox() {
    let pool = ArcPool::new();
    let mybox = Box::new("Boxed1", &pool);
    let secondbox = Box::new("Boxed2", &mybox);
    assert_ne!(mybox, secondbox);
}

#[test]
#[cfg_attr(miri, ignore)] // miri would report leaked memory here
fn forget() {
    let pool = ArcPool::new();
    Box::forget(Box::new("Boxed", &pool));
}

#[test]
fn into_inner() {
    let pool = ArcPool::new();
    let v: &str = Box::into_inner(Box::new("Was Boxed", &pool));
    assert_eq!(v, "Was Boxed");
}

#[test]
fn deref() {
    let pool = ArcPool::new();
    let mybox = Box::new("Boxed", &pool);
    assert_eq!(*mybox, "Boxed");
}

#[test]
fn deref_mut() {
    let pool = ArcPool::new();
    let mut mybox = Box::new("Boxed", &pool);
    *mybox = "Changed";
    assert_eq!(*mybox, "Changed");
}

#[test]
fn eq() {
    let pool = ArcPool::new();
    let box1 = Box::new("Boxed", &pool);
    let box2 = Box::new("Boxed", &pool);
    let box3 = Box::new("Boxed again", &pool);
    assert_eq!(box1, box2);
    assert_ne!(box1, box3);
}
