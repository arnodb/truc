#[derive(Debug)]
pub enum MachinEnum {
    Number(usize),
    Text(String),
}

impl Drop for MachinEnum {
    fn drop(&mut self) {
        match self {
            Self::Number(num) => println!("Drop num {}", num),
            Self::Text(text) => println!("Drop text \"{}\"", text),
        }
    }
}
