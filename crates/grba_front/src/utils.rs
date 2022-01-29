pub trait BoolUtils {
    fn toggle(&mut self);
}

impl BoolUtils for bool {
    fn toggle(&mut self) {
        *self = !*self;
    }
}
