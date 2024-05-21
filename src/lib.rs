pub fn add(left: usize, right: usize) -> usize {
    left + right
}

pub fn greet(name: &str) -> String {
    format!("Hello {}! The ESP instalation manager is greeting you!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn greeting_contains_name() {
        let result = greet("Carol");
        assert!(
            result.contains("Carol"),
            "Greeting did not contain name, value was `{}`",
            result
        );
    }
}
