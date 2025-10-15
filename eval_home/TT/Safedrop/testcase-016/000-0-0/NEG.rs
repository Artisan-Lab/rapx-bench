use std::rc::Rc;
fn main() {
    let x = Rc::new(5usize);
    let ptr = {
Rc::into_raw(x)
}; // SOURCE
    unsafe {
        Rc::decrement_strong_count(ptr); // GOOD SINK
    }
}