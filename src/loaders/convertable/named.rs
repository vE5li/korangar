pub trait Named {
    const NAME: &'static str;
}

#[cfg(test)]
mod simple {
    use std::marker::PhantomData;

    use procedural::*;

    use super::Named;

    #[test]
    fn empty_struct() {
        #[derive(Named)]
        struct EmptyStruct;

        assert_eq!(EmptyStruct::NAME, "EmptyStruct");
    }

    #[test]
    fn union_struct() {
        #[derive(Named)]
        struct UnionStruct();

        assert_eq!(UnionStruct::NAME, "UnionStruct");
    }

    #[test]
    fn field_struct() {
        #[derive(Named)]
        struct FieldStruct {
            _field: (),
        }

        assert_eq!(FieldStruct::NAME, "FieldStruct");
    }

    #[test]
    fn r#enum() {
        #[derive(Named)]
        enum Enum {
            _Variant,
        }

        assert_eq!(Enum::NAME, "Enum");
    }

    #[test]
    fn union() {
        #[derive(Named)]
        union Union {
            _first: (),
            _second: (),
        }

        assert_eq!(Union::NAME, "Union");
    }

    #[test]
    fn gnenric_struct() {
        #[derive(Named)]
        struct GenericStruct<'a, B, const C: usize> {
            _field: PhantomData<&'a [B; C]>,
        }

        assert_eq!(GenericStruct::<'static, (), 1>::NAME, "GenericStruct");
    }
}
