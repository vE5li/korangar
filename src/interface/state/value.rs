#[derive(Clone, Debug)]
pub enum StateValue {
    Boolean(bool),
}

impl StateValue {

    pub fn to_boolean(self) -> bool {
        match self {
            StateValue::Boolean(state) => return state,
        }
    }
}
