use crate::ConversionError;

/// Result type returned by any conversion operation.
pub type ConversionResult<T> = Result<T, Box<ConversionError>>;

/// Trait providing stack track helpers to [`ConversionResult`]
pub trait ConversionResultExt {
    /// Add a type name to the stack trace.
    fn trace<CALLER>(self) -> Self;
}

impl<T> ConversionResultExt for ConversionResult<T> {
    fn trace<CALLER>(self) -> Self {
        self.map_err(|mut error| {
            error.add_to_stack(std::any::type_name::<CALLER>());
            error
        })
    }
}

// #[cfg(test)]
// mod conversion_result {
//     use super::conversion_result;
//     use crate::ConversionError;
//
//     struct Dummy {}
//
//     #[test]
//     fn ok() {
//         let result = conversion_result::<Dummy, ()>(Ok(()));
//         assert!(result.is_ok());
//     }
//
//     #[test]
//     fn err() {
//         let error = Err(ConversionError::from_message("test"));
//         let result = conversion_result::<Dummy, ()>(error);
//
//         assert!(result.is_err());
//         assert!(format!("{:?}",
// result.unwrap_err()).contains(std::any::type_name::<Dummy>()));     }
// }
