pub type Value = f64;

// A list of the values that appear as literals in the program
#[derive(Debug)]
pub struct ValueArray {
    pub values: Vec<Value>,
}

impl ValueArray {
    pub fn new() -> Self {
        Self {
            values: vec![],
        }
    }
    pub fn write(&mut self, val: Value) {
        self.values.push(val);
    }
}
