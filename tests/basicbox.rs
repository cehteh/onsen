use onsen::*;

#[test]
fn smoke() {
    let pool = Pool::new();
    let bbox: BasicBox<u64, _> = BasicBox::default(&pool);
    BasicBox::drop(bbox, &pool)
}

#[test]
fn alloc_access() {
    let pool = Pool::new();
    let bbox = BasicBox::new("Hello Memory", &pool);
    assert_eq!(*bbox, "Hello Memory");
    BasicBox::drop(bbox, &pool)
}

#[test]
fn alloc_mutate() {
    let pool = Pool::new();

    let mut bbox = BasicBox::new(12345, &pool);

    assert_eq!(*bbox, 12345);
    *bbox = 54321;
    assert_eq!(*bbox, 54321);
    BasicBox::drop(bbox, &pool)
}
