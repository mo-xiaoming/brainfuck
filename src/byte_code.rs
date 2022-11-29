use crate::source_file::SourceFileLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub(crate) enum ByteCodeKind {
    IncPtr,
    DecPtr,
    IncData,
    DecData,
    Read,
    Write,
    LoopStartJumpIfDataZero,
    LoopEndJumpIfDataNotZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct ByteCode<'src_file> {
    pub(crate) kind: ByteCodeKind,
    pub(crate) arg: usize,
    pub(crate) range: (SourceFileLocation<'src_file>, SourceFileLocation<'src_file>),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        is_small_value_enum(&ByteCodeKind::DecData);
    }
}
