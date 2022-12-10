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
fn make_mock_byte_code(src_file: &crate::source_file::SourceFile) -> ByteCode {
    let loc = crate::source_file::make_mock_src_file_loc(src_file);
    ByteCode {
        kind: ByteCodeKind::DecData,
        arg: 1,
        range: (loc, loc),
    }
}

#[cfg(test)]
mod test {
    use crate::source_file::make_mock_src_file;

    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        is_small_value_enum(&ByteCodeKind::DecData);

        let src_file = make_mock_src_file();
        is_big_value_struct_but_no_default(&make_mock_byte_code(&src_file));
    }
}
