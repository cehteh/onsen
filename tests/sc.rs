use onsen::*;

#[test]
fn smoke() {
    let pool = Pool::new();
    let _mysc = Sc::new("Sc", &pool);
}

#[test]
fn clone() {
    let pool = Pool::new();
    let mysc1 = Sc::new("Sc", &pool);
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
    let mut mysc = Sc::new("Sc", &pool);
    *mysc = "Changed";
    assert_eq!(*mysc, "Changed");
}
