use onsen::*;

#[test]
fn smoke() {
    let pool: Pool<&str> = Pool::new();
    let memory = pool.alloc("Hello Memory");
    pool.dealloc(memory)
}

#[test]
fn alloc_access() {
    let pool: Pool<&str> = Pool::new();

    let memory = pool.alloc("Hello Memory");

    assert_eq!(*memory, "Hello Memory");
    pool.dealloc(memory)
}

#[test]
fn alloc_mutate() {
    let pool: Pool<u64> = Pool::new();

    let mut memory = pool.alloc(12345);

    assert_eq!(*memory, 12345);
    *memory = 54321;
    assert_eq!(*memory, 54321);
    pool.dealloc(memory)
}
