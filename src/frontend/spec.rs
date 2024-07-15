pub enum Target {
    C,
    Java,
    Rust,
}

pub struct Values {
    pub values: Vec<String>,
    pub value_type: Option<String>,
    pub default_value: String,
}

pub struct Spec {
    pub keys: Vec<Vec<u32>>,
    pub values: Option<Values>,
    pub target: Target,
    pub parameters_only: bool,
    pub search_width: u32,
}
