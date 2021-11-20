#[derive(Clone, Debug)]
pub enum StateValue {
    Boolean(bool),
    Number(usize),
}

impl StateValue {

    pub fn to_boolean(self) -> bool {
        match self {
            StateValue::Boolean(state) => return state,
            _invalid => panic!("invalid key type"),
        }
    }

    pub fn to_number(self) -> usize {
        match self {
            StateValue::Number(number) => return number,
            _invalid => panic!("invalid key type"),
        }
    }
}
