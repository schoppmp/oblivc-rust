extern crate libobliv_sys;

mod compiler;
pub use compiler::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_builder() {
        let _ = compiler::new_builder();
    }
}
