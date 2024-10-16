#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionErrorType {
    ByteReaderTooShort { type_name: &'static str },
    DataTooBig { type_name: &'static str },
    IncorrectMetadata { type_name: &'static str },
    Specific { message: String },
}

#[derive(Clone, PartialEq, Eq)]
pub struct ConversionError {
    error_type: ConversionErrorType,
    stack: Vec<&'static str>,
}

impl ConversionError {
    pub fn from_error_type(error_type: ConversionErrorType) -> Box<Self> {
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

    pub fn is_byte_reader_too_short(&self) -> bool {
        matches!(self.error_type, ConversionErrorType::ByteReaderTooShort { .. })
    }

    fn add_to_stack(&mut self, type_name: &'static str) {
        self.stack.insert(0, type_name);
    }
}

impl std::fmt::Debug for ConversionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stack = self.stack.join("::");

        match &self.error_type {
            ConversionErrorType::ByteReaderTooShort { type_name } => {
                write!(formatter, "byte stream too short while parsing {} in {}", type_name, stack)
            }
            ConversionErrorType::DataTooBig { type_name } => {
                write!(
                    formatter,
                    "data is too big for the available space for {} in {}",
                    type_name, stack
                )
            }
            ConversionErrorType::IncorrectMetadata { type_name } => {
                write!(
                    formatter,
                    "the metadata associated to the byte stream is incorrect for {} in {}",
                    type_name, stack
                )
            }
            ConversionErrorType::Specific { message } => write!(formatter, "{} in {}", message, stack),
        }
    }
}

/// Result type returned by any conversion operation.
pub type ConversionResult<T> = Result<T, Box<ConversionError>>;

/// Trait providing stack track helpers to [`ConversionResult`]
pub trait ConversionResultExt {
    /// Add a type name to the stack trace.
    fn trace<Caller>(self) -> Self;
}

impl<T> ConversionResultExt for ConversionResult<T> {
    fn trace<Caller>(self) -> Self {
        self.map_err(|mut error| {
            error.add_to_stack(std::any::type_name::<Caller>());
            error
        })
    }
}

#[cfg(test)]
mod conversion_result {
    use super::{ConversionError, ConversionResult, ConversionResultExt};

    struct Dummy {}

    #[test]
    fn trace_ok() {
        let result = Ok(()).trace::<Dummy>();
        assert!(result.is_ok());
    }

    #[test]
    fn trace_err() {
        let result: ConversionResult<()> = Err(ConversionError::from_message("test"));
        let result = result.trace::<Dummy>();

        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains(std::any::type_name::<Dummy>()));
    }
}

#[cfg(test)]
mod instanciate {
    use super::{ConversionError, ConversionErrorType};

    #[test]
    fn from_error_type() {
        let error_type = ConversionErrorType::ByteReaderTooShort { type_name: "test" };
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
        let error_type = ConversionErrorType::ByteReaderTooShort { type_name: "test" };
        let mut error = ConversionError::from_error_type(error_type.clone());

        error.add_to_stack(FIRST);

        assert_eq!(error.stack, vec![FIRST]);
    }

    #[test]
    fn multiple() {
        let error_type = ConversionErrorType::ByteReaderTooShort { type_name: "test" };
        let mut error = ConversionError::from_error_type(error_type.clone());

        error.add_to_stack(THIRD);
        error.add_to_stack(SECOND);
        error.add_to_stack(FIRST);

        assert_eq!(error.stack, vec![FIRST, SECOND, THIRD]);
    }
}

#[cfg(test)]
mod type_check {
    use super::{ConversionError, ConversionErrorType};

    #[test]
    fn is_byte_reader_too_short() {
        let error_type = ConversionErrorType::ByteReaderTooShort { type_name: "test" };
        let error = ConversionError::from_error_type(error_type.clone());

        assert!(error.is_byte_reader_too_short());
    }
}
