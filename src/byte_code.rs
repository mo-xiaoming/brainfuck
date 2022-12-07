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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct ByteCode<'src_file> {
    pub(crate) kind: ByteCodeKind,
    pub(crate) arg: usize,
    pub(crate) range: (SourceFileLocation<'src_file>, SourceFileLocation<'src_file>),
}

#[cfg(test)]
mod test {
    use crate::source_file::SourceFile;

    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        is_small_value_enum(&ByteCodeKind::DecData);

        let src_file = SourceFile {
            filename: std::path::PathBuf::new(),
            raw_content: String::new(),
            content: vec![],
        };
        let src_file_loc = SourceFileLocation {
            src_file: &src_file,
            row: 1,
            column: 1,
            offset: 1,
        };
        is_big_value_struct_but_no_default(&ByteCode {
            kind: ByteCodeKind::DecData,
            arg: 1,
            range: (src_file_loc, src_file_loc),
        });
    }
}
