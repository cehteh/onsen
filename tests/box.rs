use onsen::*;

#[test]
fn smoke() {
    let pool: Pool<&str> = Pool::new();
    let _mybox = pool.alloc_box("Boxed");
}

#[test]
fn deref() {
    let pool: Pool<&str> = Pool::new();
    let mybox = pool.alloc_box("Boxed");
    assert_eq!(*mybox, "Boxed");
}

#[test]
fn deref_mut() {
    let pool: Pool<&str> = Pool::new();
    let mut mybox = pool.alloc_box("Boxed");
    *mybox = "Changed";
    assert_eq!(*mybox, "Changed");
}
