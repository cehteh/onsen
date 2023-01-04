use std::io::Write;

use onsen::*;

fn init() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LOGGER: std::sync::Once = std::sync::Once::new();

    let counter = AtomicU64::new(0);
    let seq_num = move || counter.fetch_add(1, Ordering::SeqCst);

    LOGGER.call_once(|| {
        env_logger::Builder::from_default_env()
            .format(move |buf, record| {
                writeln!(
                    buf,
                    "{:0>12}: {:>5}: {}:{}: {}: {}",
                    seq_num(),
                    record.level().as_str(),
                    record.file().unwrap_or(""),
                    record.line().unwrap_or(0),
                    std::thread::current().name().unwrap_or("UNKNOWN"),
                    record.args()
                )
            })
            .try_init()
            .unwrap();
    });

    init_segv_handler();
}

fn init_segv_handler() {
    use libc::*;
    unsafe extern "C" fn handler(signum: c_int) {
        let mut sigs = std::mem::MaybeUninit::uninit();
        sigemptyset(sigs.as_mut_ptr());
        sigaddset(sigs.as_mut_ptr(), signum);
        sigprocmask(SIG_UNBLOCK, sigs.as_ptr(), std::ptr::null_mut());
        panic!("SEGV!");
    }
    unsafe {
        signal(SIGSEGV, handler as sighandler_t);
    }
}

#[test]
fn smoke() {
    init();
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
