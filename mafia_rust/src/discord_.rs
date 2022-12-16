pub mod types {
    use std::fmt::Display;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UserID(pub u64);

    impl Display for UserID {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "uid_{}", self.0)
        }
    }
}
