pub trait OptionUpdate<T: Clone> {
    fn update_if_none(&mut self, other: Option<T>);
    fn update_if_none_clone(&mut self, other: Option<&T>);
    fn update_if_none_with<F>(&mut self, other: F)
    where
        F: FnOnce() -> Option<T>;
}

impl<T: Clone> OptionUpdate<T> for Option<T> {
    fn update_if_none(&mut self, other: Option<T>) {
        if self.is_none() {
            *self = other;
        }
    }

    fn update_if_none_clone(&mut self, other: Option<&T>) {
        if self.is_none() {
            *self = other.cloned();
        }
    }

    fn update_if_none_with<F>(&mut self, other: F)
    where
        F: FnOnce() -> Option<T>
    {
        if self.is_none() {
            *self = other();
        }
    }
}
