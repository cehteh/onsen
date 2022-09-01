use onsen::*;
use serial_test::serial;

struct Test;
define_tbox_pool!(Test: &'static str);

#[test]
#[serial]
fn smoke() {
    TBox::<&'static str, Test>::pool().acquire().unwrap();
    {
        let _mybox = TBox::new("Boxed", Test);
    }
    TBox::<&'static str, Test>::pool().release().unwrap();
}

#[test]
#[serial]
fn deref() {
    TBox::<&'static str, Test>::pool().acquire().unwrap();
    {
        let mybox = TBox::new("Boxed", Test);
        assert_eq!(*mybox, "Boxed");
    }
    TBox::<&'static str, Test>::pool().release().unwrap();
}

#[test]
#[serial]
fn deref_mut() {
    TBox::<&'static str, Test>::pool().acquire().unwrap();
    {
        let mut mybox = TBox::new("Boxed", Test);
        *mybox = "Changed";
        assert_eq!(*mybox, "Changed");
    }
    TBox::<&'static str, Test>::pool().release().unwrap();
}

#[test]
#[serial]
fn eq() {
    TBox::<&'static str, Test>::pool().acquire().unwrap();
    {
        let box1 = TBox::new("Boxed", Test);
        let box2 = TBox::new("Boxed", Test);
        let box3 = TBox::new("Boxed again", Test);
        assert_eq!(box1, box2);
        assert_ne!(box1, box3);
    }
    TBox::<&'static str, Test>::pool().release().unwrap();
}
