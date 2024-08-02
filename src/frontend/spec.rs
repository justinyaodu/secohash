pub enum Target {
    C,
    Java,
    Rust,
}

//                          valid key   invalid key
// hash(key)                its hash    its hash
// hash_valid(key, hash)    true        false
// key_valid(key)           true        false
// lookup_foo(key)          &value      null

// --hash-function-name
// --key-table-name
// --contains-function-name
// --lookup-function-names
// --value-table-names
// --value-types
// --target
// k v1 v2 k v1 v2 vs k k v1 v1 v2 v2
// --input-style={interleaved,grouped}

pub struct Values {
    pub values: Vec<String>,
    pub value_type: Option<String>,
    pub default_value: String,
}

pub struct Spec {
    pub keys: Vec<Vec<u32>>,
    pub values: Option<Values>,
    pub target: Target,

    pub interpreted_keys: Vec<Vec<u32>>,
    pub min_interpreted_key_len: usize,
    pub max_interpreted_key_len: usize,
    pub min_hash_bits: u32,
}
