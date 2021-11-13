//! blocknative-rs
//!
//! Rust library for blocknative api
pub mod models;
pub mod ws;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
