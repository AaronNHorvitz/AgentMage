#![forbid(unsafe_code)]

fn main() {}

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_exposes_no_release_action() {
        assert!(std::env::args_os().next().is_some());
    }
}
