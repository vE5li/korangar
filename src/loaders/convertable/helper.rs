use super::{ConversionError, ConversionErrorType, Named};

#[inline(always)]
pub fn check_upper_bound<S: Named>(offset: usize, length: usize) -> Result<(), Box<ConversionError>> {
    match offset < length {
        true => Ok(()),
        false => Err(ConversionError::from_error_type(ConversionErrorType::ByteStreamTooShort {
            type_name: S::NAME,
        })),
    }
}

#[inline(always)]
pub fn conversion_result<S: Named, T>(result: Result<T, Box<ConversionError>>) -> Result<T, Box<ConversionError>> {
    result.map_err(|mut error| {
        error.add_to_stack(S::NAME);
        error
    })
}

#[cfg(test)]
mod length_hint_none {
    use procedural::Named;

    use super::check_length_hint_none;

    #[derive(Named)]
    struct Dummy {}

    #[test]
    fn none() {
        let result = check_length_hint_none::<Dummy>(None);
        assert!(result.is_ok());
    }

    #[test]
    fn some() {
        let result = check_length_hint_none::<Dummy>(Some(0));
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod length_hint {
    use procedural::Named;

    use super::check_length_hint;

    #[derive(Named)]
    struct Dummy {}

    #[test]
    fn none() {
        let result = check_length_hint::<Dummy>(None);
        assert!(result.is_err());
    }

    #[test]
    fn some() {
        const LENGTH_HINT: usize = 0;
        let result = check_length_hint::<Dummy>(Some(LENGTH_HINT));
        assert!(result.is_ok_and(|value| value == LENGTH_HINT));
    }
}

#[cfg(test)]
mod upper_bound {
    use procedural::Named;

    use super::check_upper_bound;

    #[derive(Named)]
    struct Dummy {}

    #[test]
    fn smaller() {
        let result = check_upper_bound::<Dummy>(5, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn equals() {
        let result = check_upper_bound::<Dummy>(10, 10);
        assert!(result.is_err());
    }

    #[test]
    fn greater() {
        let result = check_upper_bound::<Dummy>(15, 10);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod conversion_result {
    use procedural::Named;

    use super::conversion_result;
    use crate::loaders::{ConversionError, Named};

    #[derive(Named)]
    struct Dummy {}

    #[test]
    fn ok() {
        let result = conversion_result::<Dummy, ()>(Ok(()));
        assert!(result.is_ok());
    }

    #[test]
    fn err() {
        let error = Err(ConversionError::from_message("test"));
        let result = conversion_result::<Dummy, ()>(error);

        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains(Dummy::NAME));
    }
}
