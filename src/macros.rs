// Helper macros for boilerplate code

#[doc(hidden)]
#[macro_export]
macro_rules! hasher_impl {
    () => {
        #[mutants::skip] /* we just pretend it works */
        fn finish(&self) -> u64 {
            (**self).finish()
        }
        #[mutants::skip] /* we just pretend it works */
        fn write(&mut self, bytes: &[u8]) {
            (**self).write(bytes);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_u8(&mut self, i: u8) {
            (**self).write_u8(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_u16(&mut self, i: u16) {
            (**self).write_u16(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_u32(&mut self, i: u32) {
            (**self).write_u32(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_u64(&mut self, i: u64) {
            (**self).write_u64(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_u128(&mut self, i: u128) {
            (**self).write_u128(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_usize(&mut self, i: usize) {
            (**self).write_usize(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_i8(&mut self, i: i8) {
            (**self).write_i8(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_i16(&mut self, i: i16) {
            (**self).write_i16(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_i32(&mut self, i: i32) {
            (**self).write_i32(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_i64(&mut self, i: i64) {
            (**self).write_i64(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_i128(&mut self, i: i128) {
            (**self).write_i128(i);
        }
        #[mutants::skip] /* we just pretend it works */
        fn write_isize(&mut self, i: isize) {
            (**self).write_isize(i);
        }
        // fn write_length_prefix(&mut self, len: usize) {
        //     (**self).write_length_prefix(len)
        // }
        // fn write_str(&mut self, s: &str) {
        //     (**self).write_str(s)
        // }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! partial_ord_impl {
    () => {
        #[mutants::skip] /* we just pretend it works */
        #[inline]
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            PartialOrd::partial_cmp(&**self, &**other)
        }
        #[mutants::skip] /* we just pretend it works */
        #[inline]
        fn lt(&self, other: &Self) -> bool {
            PartialOrd::lt(&**self, &**other)
        }
        #[mutants::skip] /* we just pretend it works */
        #[inline]
        fn le(&self, other: &Self) -> bool {
            PartialOrd::le(&**self, &**other)
        }
        #[mutants::skip] /* we just pretend it works */
        #[inline]
        fn ge(&self, other: &Self) -> bool {
            PartialOrd::ge(&**self, &**other)
        }
        #[mutants::skip] /* we just pretend it works */
        #[inline]
        fn gt(&self, other: &Self) -> bool {
            PartialOrd::gt(&**self, &**other)
        }
    }
}
