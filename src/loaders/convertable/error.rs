#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionErrorType {
    UnusedLengthHint { type_name: &'static str, length_hint: usize },
    MissingLengthHint { type_name: &'static str },
    ByteStreamTooShort { type_name: &'static str },
    Specific { message: String },
}

#[derive(Clone)]
pub struct ConversionError {
    error_type: ConversionErrorType,
    stack: Vec<&'static str>,
}

impl ConversionError {
    pub(super) fn from_error_type(error_type: ConversionErrorType) -> Box<Self> {
        Box::new(Self {
            error_type,
            stack: Vec::new(),
        })
    }

    pub fn from_message(message: impl ToString) -> Box<Self> {
        Self::from_error_type(ConversionErrorType::Specific {
            message: message.to_string(),
        })
    }

    pub(super) fn add_to_stack(&mut self, type_name: &'static str) {
        self.stack.insert(0, type_name);
    }
}

impl std::fmt::Debug for ConversionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stack = self.stack.join("::");

        match &self.error_type {
            ConversionErrorType::UnusedLengthHint { type_name, length_hint } => write!(
                formatter,
                "unused length hint ({}) while parsing {} in {}",
                length_hint, type_name, stack
            ),
            ConversionErrorType::MissingLengthHint { type_name } => {
                write!(formatter, "missing length hint while parsing {} in {}", type_name, stack)
            }
            ConversionErrorType::ByteStreamTooShort { type_name } => {
                write!(formatter, "byte stream too short while parsing {} in {}", type_name, stack)
            }
            ConversionErrorType::Specific { message } => write!(formatter, "{} in {}", message, stack),
        }
    }
}

#[cfg(test)]
mod instanciate {
    use super::{ConversionError, ConversionErrorType};

    #[test]
    fn from_error_type() {
        let error_type = ConversionErrorType::ByteStreamTooShort { type_name: "test" };
        let error = ConversionError::from_error_type(error_type.clone());

        assert_eq!(error.error_type, error_type);
        assert!(error.stack.is_empty());
    }

    #[test]
    fn from_message() {
        let message = "test".to_owned();
        let error = ConversionError::from_message(message.clone());

        assert_eq!(error.error_type, ConversionErrorType::Specific { message });
        assert!(error.stack.is_empty());
    }
}

#[cfg(test)]
mod add_to_stack {
    use super::{ConversionError, ConversionErrorType};

    const FIRST: &str = "first";
    const SECOND: &str = "second";
    const THIRD: &str = "third";

    #[test]
    fn empty() {
        let error_type = ConversionErrorType::ByteStreamTooShort { type_name: "test" };
        let mut error = ConversionError::from_error_type(error_type.clone());

        error.add_to_stack(FIRST);

        assert_eq!(error.stack, vec![FIRST]);
    }

    #[test]
    fn multiple() {
        let error_type = ConversionErrorType::ByteStreamTooShort { type_name: "test" };
        let mut error = ConversionError::from_error_type(error_type.clone());

        error.add_to_stack(THIRD);
        error.add_to_stack(SECOND);
        error.add_to_stack(FIRST);

        assert_eq!(error.stack, vec![FIRST, SECOND, THIRD]);
    }
}
