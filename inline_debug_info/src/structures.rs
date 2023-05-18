#[derive(Debug, Serialize, Deserialize)]
struct Function {
    name: String,
    relative_start: u64,
    start: u64,
    len: u64,
    frame_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Region {
    start: u64,
    len: u64,
    file: String,
    line: u64,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct DebugInfo {
    functions: Vec<Function>,
    regions: Vec<Region>,
}
