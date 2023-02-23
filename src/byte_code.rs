use crate::source_file::{RawContentIndex, UcContentIndex};

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
pub struct ByteCode {
    pub(crate) kind: ByteCodeKind,
    pub(crate) arg: usize,
    pub(crate) range_in_raw: (RawContentIndex, RawContentIndex),
}
#[cfg(test)]
fn make_mock_byte_code() -> ByteCode {
    ByteCode {
        kind: ByteCodeKind::DecData,
        arg: 1,
        range_in_raw: (RawContentIndex::new(0), RawContentIndex::new(0)),
    }
}

impl ByteCode {
    pub(crate) fn make_non_jump_code(
        kind: ByteCodeKind,
        idx_in_raw: RawContentIndex,
        raw_len: usize,
    ) -> Self {
        Self {
            kind,
            arg: raw_len,
            range_in_raw: (idx_in_raw, RawContentIndex::inc_from(idx_in_raw, raw_len)),
        }
    }

    pub(crate) fn make_uninit_jump_code(kind: ByteCodeKind, idx_in_raw: RawContentIndex) -> Self {
        Self {
            kind,
            arg: usize::MAX,
            range_in_raw: (idx_in_raw, RawContentIndex::inc_from(idx_in_raw, 1)),
        }
    }

    pub(crate) fn correct_jump(&mut self, jump_to: UcContentIndex) {
        self.arg = jump_to.get();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        is_small_value_enum(&ByteCodeKind::DecData);

        is_big_value_struct_but_no_default(&make_mock_byte_code());
    }
}
