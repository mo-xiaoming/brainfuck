pub trait MachineIO {
    fn out_char_n_times(&mut self, c: char, n: usize);
    fn in_char(&mut self) -> char;
    fn flush_all(&mut self);
}

#[derive(Debug)]
pub struct DefaultMachineIO {
    term: console::Term,
}

impl Default for DefaultMachineIO {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultMachineIO {
    pub fn new() -> Self {
        Self {
            term: console::Term::stdout(),
        }
    }
}

impl MachineIO for DefaultMachineIO {
    fn out_char_n_times(&mut self, c: char, n: usize) {
        print!("{}", c.to_string().repeat(n));
    }

    fn in_char(&mut self) -> char {
        self.term.read_char().unwrap()
    }

    fn flush_all(&mut self) {}
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        is_default_debug(&DefaultMachineIO::default());
    }
}
