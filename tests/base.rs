use onsen::*;

#[test]
fn smoke() {
    let pool: Pool<&str> = Pool::new();

    let memory = pool.alloc("Hello Memory");
    unsafe {
        pool.free(memory);
    }
}

#[test]
fn leak() {
    let pool: Pool<&str> = Pool::new();

    let _memory = pool.alloc("Hello Memory");

    pool.leak();
}

#[test]
fn alloc_access() {
    let pool: Pool<&str> = Pool::new();

    let mut memory = pool.alloc("Hello Memory").for_mutation();

    assert_eq!(memory.get(), &"Hello Memory");
    assert_eq!(memory.get_mut(), &"Hello Memory");

    unsafe {
        pool.free(memory);
    }
}

#[test]
fn alloc_uninit() {
    let pool: Pool<&str> = Pool::new();

    let mut memory = pool.alloc_uninit();

    let memory = unsafe {
        memory.get_uninit().write("Hello Init");
        memory.assume_init()
    };

    assert_eq!(memory.get(), &"Hello Init");

    unsafe {
        pool.free(memory);
    }
}
