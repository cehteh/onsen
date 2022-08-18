use onsen::*;

#[test]
fn smoke() {
    let pool = Pool::new();
    let _myrc = pool.alloc_rc("Rc");
}

#[test]
fn macro_test() {
    let pool = Pool::new();
    let myrc = pool.alloc_rc("Rc");
    assert_eq!(*myrc, "Rc");
}

#[test]
fn clone() {
    let pool = Pool::new();
    let myrc1 = pool.alloc_rc("Rc");
    let myrc2 = myrc1.clone();
    let myrc3 = Rc::clone(&myrc2);

    assert_eq!(*myrc1, "Rc");
    assert_eq!(myrc1, myrc2);
    assert_eq!(myrc2, myrc3);
    assert_eq!(Rc::strong_count(&myrc3), 3);
}

#[test]
fn deref_mut() {
    let pool = Pool::new();
    let mut myrc = pool.alloc_rc("Rc");
    *myrc = "Changed";
    assert_eq!(*myrc, "Changed");
}

#[test]
fn weak() {
    let pool = Pool::new();
    let myrc = pool.alloc_rc("Rc");
    let weak = Rc::downgrade(&myrc);
    assert_eq!(weak.strong_count(), 1);
    assert_eq!(weak.weak_count(), 1);
    let strong = weak.upgrade().unwrap();
    assert_eq!(Rc::strong_count(&strong), 2);
    assert_eq!(myrc, strong);
    assert_eq!(*strong, "Rc");
}
