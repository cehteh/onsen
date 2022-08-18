use onsen::*;

#[test]
fn smoke() {
    let pool = Pool::new();
    let _mysc = pool.alloc_sc("Sc");
}

#[test]
fn macro_test() {
    let pool = Pool::new();
    let mysc = pool.alloc_sc("Sc");
    assert_eq!(*mysc, "Sc");
}

#[test]
fn clone() {
    let pool = Pool::new();
    let mysc1 = pool.alloc_sc("Sc");
    let mysc2 = mysc1.clone();
    let mysc3 = Sc::clone(&mysc2);

    assert_eq!(*mysc1, "Sc");
    assert_eq!(mysc1, mysc2);
    assert_eq!(mysc2, mysc3);
    assert_eq!(Sc::strong_count(&mysc3), 3);
}

#[test]
fn deref_mut() {
    let pool = Pool::new();
    let mut mysc = pool.alloc_sc("Sc");
    *mysc = "Changed";
    assert_eq!(*mysc, "Changed");
}
