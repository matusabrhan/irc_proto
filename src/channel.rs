#[derive(Debug)]
pub struct Channel {
    pub name: String,
    pub members: Vec<String>,
    pub flags: String,
}

impl Channel {
    pub fn new(name: String, member: String) -> Channel {
        Channel {
            name,
            members: Vec::from([member]),
            flags: String::new(),
        }
    }
}
