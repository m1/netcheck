use std::fmt::Display;

/// Convert a vector to a vector of strings.
///
/// # Arguments
///
/// * `vec`: vector of T
///
/// returns: Vec<String, Global>
pub fn vec_to_strings<T: Display>(vec: Vec<T>) -> Vec<String> {
    vec.iter().map(|x| x.to_string()).collect()
}

/// Convert a vector to a string.
///
/// # Arguments
///
/// * `vec`: vector of T
///
/// returns: String
///
pub fn vec_to_string<T: Display>(vec: Vec<T>) -> String {
    vec_to_strings(vec).join(",")
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_vec_to_strings() {
        let vec = vec![1, 2, 3];
        assert_eq!(vec_to_strings(vec), vec!["1".to_string(), "2".to_string(), "3".to_string()]);
    }

    #[test]
    fn test_vec_to_string() {
        let vec = vec![1, 2, 3];
        assert_eq!(vec_to_string(vec), "1,2,3".to_string());
    }
}
