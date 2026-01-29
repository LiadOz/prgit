use crate::error::{P4Error, P4Message};
use std::ops::Index;

#[derive(Debug, Clone)]
pub struct P4Output<T> {
    pub results: Vec<T>,
    pub warnings: Vec<P4Message>,
}

impl<T> Index<usize> for P4Output<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.results[index]
    }
}

impl<T> P4Output<T> {
    pub fn new(results: Vec<T>, warnings: Vec<P4Message>) -> Self {
        Self { results, warnings }
    }

    pub fn empty() -> Self {
        Self { results: Vec::new(), warnings: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    #[must_use]
    pub fn single(mut self) -> Result<T, P4Error> {
        if self.results.is_empty() {
            return Err(P4Error::UnexpectedError("Expected single result, got none".into()));
        }
        if self.results.len() > 1 {
            return Err(P4Error::UnexpectedError(
                format!("Expected single result, got {}", self.results.len())
            ));
        }
        Ok(self.results.remove(0))
    }

    #[must_use]
    pub fn first(mut self) -> Option<T> {
        if self.results.is_empty() {
            None
        } else {
            Some(self.results.remove(0))
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.results.iter()
    }
}

impl<T> IntoIterator for P4Output<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.results.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a P4Output<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.results.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_success() {
        let output = P4Output::new(vec![42], vec![]);
        assert_eq!(output.single().unwrap(), 42);
    }

    #[test]
    fn test_single_empty_error() {
        let output: P4Output<i32> = P4Output::empty();
        assert!(output.single().is_err());
    }

    #[test]
    fn test_single_multiple_error() {
        let output = P4Output::new(vec![1, 2], vec![]);
        assert!(output.single().is_err());
    }

    #[test]
    fn test_first() {
        let output = P4Output::new(vec![1, 2, 3], vec![]);
        assert_eq!(output.first(), Some(1));

        let empty: P4Output<i32> = P4Output::empty();
        assert_eq!(empty.first(), None);
    }

    #[test]
    fn test_len_and_is_empty() {
        let output = P4Output::new(vec![1, 2, 3], vec![]);
        assert_eq!(output.len(), 3);
        assert!(!output.is_empty());

        let empty: P4Output<i32> = P4Output::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_into_iterator() {
        let output = P4Output::new(vec![1, 2, 3], vec![]);
        let sum: i32 = output.into_iter().sum();
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_iter() {
        let output = P4Output::new(vec![1, 2, 3], vec![]);
        let sum: i32 = output.iter().sum();
        assert_eq!(sum, 6);
    }
}
