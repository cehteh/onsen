use onsen::*;

#[test]
fn smoke() {
    let pool: Pool<&str> = Pool::new();
    let _mybox = Box::new("Boxed", &pool);
}

#[test]
fn deref() {
    let pool: Pool<&str> = Pool::new();
    let mybox = Box::new("Boxed", &pool);
    assert_eq!(*mybox, "Boxed");
}

#[test]
fn deref_mut() {
    let pool: Pool<&str> = Pool::new();
    let mut mybox = Box::new("Boxed", &pool);
    *mybox = "Changed";
    assert_eq!(*mybox, "Changed");
}

#[test]
fn eq() {
    let pool: Pool<&str> = Pool::new();
    let box1 = Box::new("Boxed", &pool);
    let box2 = Box::new("Boxed", &pool);
    let box3 = Box::new("Boxed again", &pool);
    assert_eq!(box1, box2);
    assert_ne!(box1, box3);
}
