use crate::{
    byte_code::{ByteCode, ByteCodeKind},
    source_file::RawToken,
};

pub(crate) trait LoopCode {
    fn is_loop_start(&self) -> bool;
    fn is_loop_end(&self) -> bool;
}

impl<'src_file> LoopCode for &'src_file ByteCode<'src_file> {
    fn is_loop_start(&self) -> bool {
        self.kind == ByteCodeKind::LoopStartJumpIfDataZero
    }

    fn is_loop_end(&self) -> bool {
        self.kind == ByteCodeKind::LoopEndJumpIfDataNotZero
    }
}

impl<'src_file> LoopCode for &'src_file RawToken {
    fn is_loop_start(&self) -> bool {
        self.uc == "["
    }

    fn is_loop_end(&self) -> bool {
        self.uc == "]"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub(crate) enum ExtraParen {
    Left(usize),
    Right(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LoopMatches {
    start_to_end: std::collections::HashMap<usize, usize>,
    end_to_start: std::collections::HashMap<usize, usize>,
}
#[cfg(test)]
fn make_mock_loop_matches() -> LoopMatches {
    LoopMatches {
        start_to_end: std::collections::HashMap::new(),
        end_to_start: std::collections::HashMap::new(),
    }
}

impl LoopMatches {
    pub(crate) fn get_matching_start(&self, end: usize) -> usize {
        *self.end_to_start.get(&end).unwrap()
    }
    pub(crate) fn get_matching_end(&self, start: usize) -> usize {
        *self.start_to_end.get(&start).unwrap()
    }
}

pub(crate) fn populate_loop_boundaries<I>(codes: I) -> Result<LoopMatches, ExtraParen>
where
    I: Iterator,
    <I as Iterator>::Item: LoopCode,
{
    use std::collections::HashMap;

    let mut start_to_end = HashMap::with_capacity(1_000);
    let mut end_to_start = HashMap::with_capacity(start_to_end.len());

    let mut starts = Vec::with_capacity(10);

    for (idx, code) in codes.enumerate() {
        if code.is_loop_start() {
            starts.push(idx);
        } else if code.is_loop_end() {
            let start_idx = starts.pop().ok_or(ExtraParen::Right(idx))?;
            let existed = start_to_end.insert(start_idx, idx);
            assert!(existed.is_none());
            let existed = end_to_start.insert(idx, start_idx);
            assert!(existed.is_none());
        }
    }

    if !starts.is_empty() {
        return Err(ExtraParen::Left(*starts.last().unwrap()));
    }

    Ok(LoopMatches {
        start_to_end,
        end_to_start,
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::source_file::SourceFile;
    use crate::utility::traits::{is_debug, is_small_value_enum};

    #[test]
    fn traits() {
        is_small_value_enum(&ExtraParen::Left(3));
        is_debug(&make_mock_loop_matches());
    }

    #[test]
    fn extra_left_parens() {
        let test_data = [
            ("[[]", 0),
            ("+[[]", 1),
            ("[]++[.[]", 4),
            ("[][[]][", 6),
            ("[][[[]][]", 2),
        ];

        for td in test_data {
            let src_file = SourceFile::from_str(td.0, "");
            let err = populate_loop_boundaries(src_file.iter());
            assert_eq!(err, Err(ExtraParen::Left(td.1)), "src: {}", td.0);
        }
    }

    #[test]
    fn extra_right_parens() {
        let test_data = [
            ("]", 0),
            (".[[]]]", 5),
            (".[[]]]]", 5),
            ("[].][][]", 3),
            ("[][][]][]", 6),
        ];

        for td in test_data {
            let src_file = SourceFile::from_str(td.0, "");
            let err = populate_loop_boundaries(src_file.iter());
            assert_eq!(err, Err(ExtraParen::Right(td.1)), "src: {}", td.0);
        }
    }

    #[test]
    fn loop_boundaries() {
        use std::collections::HashMap;

        let test_data = [
            ("[]", HashMap::from([(0, 1)]), HashMap::from([(1, 0)])),
            (
                "+[[]]",
                HashMap::from([(1, 4), (2, 3)]),
                HashMap::from([(3, 2), (4, 1)]),
            ),
            (
                "+[][]",
                HashMap::from([(1, 2), (3, 4)]),
                HashMap::from([(2, 1), (4, 3)]),
            ),
            (
                "[[+][]-]",
                HashMap::from([(0, 7), (1, 3), (4, 5)]),
                HashMap::from([(3, 1), (5, 4), (7, 0)]),
            ),
        ];

        for td in test_data {
            let src_file = SourceFile::from_str(td.0, "");
            let err = populate_loop_boundaries(src_file.iter());
            let oracle = LoopMatches {
                start_to_end: td.1,
                end_to_start: td.2,
            };
            assert_eq!(err, Ok(oracle), "src: {}", td.0);
        }
    }
}

#[cfg(feature = "instr_tracing")]
pub(crate) mod tracing {
    use smol_str::SmolStr;
    use thousands::Separable;

    #[derive(Debug)]
    pub(crate) struct InstructionTracingCollector {
        c: std::collections::HashMap<SmolStr, u128>,
    }

    #[derive(Debug)]
    struct TracingData {
        name: SmolStr,
        count: u128,
    }

    impl std::fmt::Display for TracingData {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(
                f,
                "{:10}: {:>15}",
                self.name,
                self.count.separate_with_commas()
            )
        }
    }

    impl InstructionTracingCollector {
        pub(crate) fn new(codes: &[&str]) -> Self {
            let mut c = std::collections::HashMap::with_capacity(20);
            for &n in codes {
                c.insert(SmolStr::from(n), 0);
            }
            Self { c }
        }

        pub(crate) fn add(&mut self, name: &str) {
            let name = SmolStr::from(name);
            if !self.c.contains_key(&name) {
                panic!("`{}` doesn't exist when constructing this collector", name);
            }
            self.c.entry(name).and_modify(|e| *e += 1);
        }

        pub(crate) fn finalize_to_string(&mut self) -> String {
            self.finalize(|vec| {
                let mut s = String::new();
                let mut total = 0;
                vec.iter()
                    .fold((&mut s, &mut total), |(acc_s, acc_total), (k, v)| {
                        acc_s.push_str(&format!("{:10}: {:>15}\n", k, v.separate_with_commas()));
                        *acc_total += *v;
                        (acc_s, acc_total)
                    });
                s.push_str(&format!("{:-<1$}\n", "", 27));
                s.push_str(&format!(
                    "{:10}: {:>15}\n",
                    "TOTAL",
                    total.separate_with_commas()
                ));
                s
            })
        }

        fn finalize<F, R>(&mut self, f: F) -> R
        where
            F: Fn(&[(&SmolStr, &u128)]) -> R,
        {
            let mut vec = self.c.iter().filter(|(_, &v)| v > 0).collect::<Vec<_>>();
            vec.sort_by_key(|(_, &v)| std::cmp::Reverse(v));

            let r = f(&vec);

            // reset
            self.c.values_mut().for_each(|v| *v = 0);

            r
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use crate::utility::traits::is_debug;

        #[test]
        fn traits() {
            is_debug(&InstructionTracingCollector {
                c: std::collections::HashMap::new(),
            });
            is_debug(&TracingData {
                name: smol_str::SmolStr::default(),
                count: 0,
            });
        }

        #[test]
        fn no_data_then_collect_nothing() {
            let mut collector = InstructionTracingCollector::new(&["a", "b"]);
            collector.finalize(|s| {
                assert_eq!(s.len(), 0);
            });
        }

        #[test]
        fn only_collect_one_then_show_one() {
            let mut collector = InstructionTracingCollector::new(&["a", "b"]);
            collector.add("b");
            collector.add("b");
            collector.finalize(|s| {
                assert_eq!(s.len(), 1);
                assert_eq!(s[0].0, "b");
                assert_eq!(*s[0].1, 2);
            });
        }

        #[test]
        fn name_not_in_codes_should_panic() {
            let result = std::panic::catch_unwind(|| {
                let mut collector = InstructionTracingCollector::new(&["a", "b"]);
                collector.add("c");
            });
            assert!(result.is_err());
        }
    }
}

#[cfg(feature = "instr_timing")]
pub(crate) mod timing {
    use smol_str::SmolStr;
    use thousands::Separable;

    /// data point for each instruction execution time
    ///
    /// since there could be billions of data points for one instruction,
    /// put all data points in one vector will cause oom-kill
    /// `pt` stores the latest N data points, once it exceeds certain limits,
    /// the its mean will be stored in `pt_agg`, which will be treated as the
    /// "real" data point at the data analyses phase.
    ///
    /// # Notes
    ///
    /// For collecting the residue of the data point in `pt` when analyzing data,
    /// `collect_pt` must be called first
    #[derive(Debug)]
    struct Points {
        pt: Vec<f32>,
        pt_agg: Vec<f32>,
    }

    impl Points {
        fn new() -> Self {
            Self {
                pt: Vec::with_capacity(Self::threshold()),
                pt_agg: Vec::with_capacity(5_000_000_000 / Self::threshold()),
            }
        }
        /// add a data point
        ///
        /// if `pt` threshold reaches, then data in `pt` will be collected
        /// and "moved" to `pt_agg`,
        fn add(&mut self, u: u128) {
            self.pt.push(u as f32);
            if self.pt.len() > Self::threshold() {
                self.collect_pt();
            }
        }
        /// collect all data in `pt` to `pt_agg` and clear `pt`
        fn collect_pt(&mut self) {
            if !self.pt.is_empty() {
                let mean = self.pt.iter().sum::<f32>() / self.pt.len() as f32;
                self.pt.clear();
                self.pt_agg.push(mean);
            }
        }
        fn threshold() -> usize {
            5_000
        }
    }

    #[derive(Debug)]
    struct Stat {
        name: SmolStr,
        mean: f32,
        min: u128,
        max: u128,
        std_deviation: f32,
    }

    /// collecting execution times for given instructions
    #[derive(Debug)]
    pub(crate) struct InstructionTimingCollector {
        c: std::collections::HashMap<SmolStr, Points>,
    }

    impl InstructionTimingCollector {
        /// `codes` are all "instructions" you want to collect for.
        ///
        /// If an instruction name is not given in `code`, and later used by `start` will cause
        /// assertion failure. A `BASELINE` "instruction" will be added as well, it indications
        /// the time consumption of no-op
        pub(crate) fn new(codes: &[&str]) -> Self {
            let mut c = std::collections::HashMap::with_capacity(20);
            for &n in codes.iter().chain(std::iter::once(&Self::baseline())) {
                c.insert(SmolStr::from(n), Points::new());
            }
            Self { c }
        }

        /// `name` is the instruction name given when `new` is called
        ///
        /// # Panics
        ///
        /// If the `name` does not in the `codes` during collector destruction
        pub(crate) fn start(&mut self, name: &str) -> InstructionTiming {
            let name = SmolStr::from(name);
            if !self.c.contains_key(&name) {
                panic!("`{}` doesn't exist when constructing this collector", name);
            }

            InstructionTiming {
                c: self,
                a: std::time::Instant::now(),
                name,
            }
        }

        /// this must be called before starting collection, otherwise, data points will mix up
        pub(crate) fn finalize_to_string(&mut self) -> String {
            self.finalize(|v| {
                let mut s = String::new();
                for Stat {
                    name,
                    mean,
                    min,
                    max,
                    std_deviation,
                } in v.iter()
                {
                    let cv = std_deviation / mean;
                    let cv_s = {
                        let cv_s = format!("{:>6.2}", cv);
                        if cv > 1.0 && console::Term::stdout().features().colors_supported() {
                            console::style(cv_s).red().to_string()
                        } else {
                            cv_s
                        }
                    };
                    s.push_str(&format!(
                        "{:10}: mean ={:>7} std ={:>8.2} cv ={} | min ={:7}, max ={:7}\n",
                        name,
                        (*mean as u128).separate_with_commas(),
                        std_deviation,
                        cv_s,
                        min,
                        max
                    ));
                }
                s
            })
        }

        fn finalize<F, R>(&mut self, f: F) -> R
        where
            F: Fn(&[Stat]) -> R,
        {
            (0..5_000_000).for_each(|_| {
                let _a = self.start(Self::baseline());
            });
            self.c.values_mut().for_each(Points::collect_pt);

            let r = f(&self.collect());

            // reset
            self.reset();

            r
        }

        /// reset collector for reuse
        ///
        /// # Panics
        ///
        /// If there are still data left
        fn reset(&mut self) {
            for (k, v) in self.c.iter_mut() {
                if !v.pt.is_empty() {
                    panic!(
                        "{} data points of `{}` have not been collected",
                        v.pt.len(),
                        k
                    );
                }

                v.pt.clear();
                v.pt_agg.clear();
            }
        }

        const fn baseline() -> &'static str {
            "BASELINE"
        }

        fn to_stats(name: &SmolStr, points: &[f32]) -> Option<Stat> {
            if points.is_empty() {
                return None;
            }

            let (min, max) = (
                points.iter().map(|&e| e as u128).min().unwrap(),
                points.iter().map(|&e| e as u128).max().unwrap(),
            );
            let mean = points.iter().sum::<f32>() / points.len() as f32;
            let std_deviation = {
                let variance = points
                    .iter()
                    .map(|e| {
                        let diff = mean - *e;
                        diff * diff
                    })
                    .sum::<f32>()
                    / points.len() as f32;
                variance.sqrt()
            };
            Some(Stat {
                name: name.clone(),
                mean,
                min,
                max,
                std_deviation,
            })
        }

        fn collect(&self) -> Vec<Stat> {
            self.c
                .iter()
                .filter_map(|(k, v)| Self::to_stats(k, &v.pt_agg))
                .collect()
        }
    }

    /// collects all data points for one instruction in RAII style
    #[derive(Debug)]
    pub(crate) struct InstructionTiming<'c> {
        c: &'c mut InstructionTimingCollector,
        a: std::time::Instant,
        name: SmolStr,
    }

    /// data point added during `drop`
    impl<'c> Drop for InstructionTiming<'c> {
        fn drop(&mut self) {
            self.c
                .c
                .get_mut(&self.name)
                .unwrap()
                .add(self.a.elapsed().as_nanos());
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use crate::utility::traits::{is_debug, is_small_value_enum};

        #[test]
        fn traits() {
            is_debug(&Points {
                pt: Vec::new(),
                pt_agg: Vec::new(),
            });
            is_debug(&Stat {
                name: smol_str::SmolStr::default(),
                mean: 0.0,
                min: 0,
                max: 0,
                std_deviation: 0.0,
            });
            is_debug(&InstructionTimingCollector {
                c: std::collections::HashMap::new(),
            });
        }

        #[test]
        fn start_never_called_then_only_baseline_collected() {
            let mut collector = InstructionTimingCollector::new(&["a", "b"]);
            collector.finalize(|s| {
                assert_eq!(s.len(), 1); // BASELINE
            });
        }

        #[test]
        fn only_collect_one_then_show_one() {
            let mut collector = InstructionTimingCollector::new(&["a", "b"]);
            {
                let _t = collector.start("b");
            }
            collector.finalize(|s| {
                assert_eq!(s.len(), 2); // BASELINE and b
                if s[0].name == "BASELINE" {
                    assert!(s[1].name == "b");
                } else if s[0].name == "b" {
                    assert!(s[1].name == "BASELINE");
                } else {
                    panic!("{:?} doesn't have correct values", s);
                }
            });
        }

        #[test]
        fn name_not_in_codes_should_panic() {
            let result = std::panic::catch_unwind(|| {
                let mut collector = InstructionTimingCollector::new(&["a", "b"]);
                {
                    let _t = collector.start("c");
                }
            });
            assert!(result.is_err());
        }
    }
}

#[cfg(test)]
pub(crate) mod traits {
    /// have everything a value type should have, but no meaningful default
    pub(crate) fn is_small_value_struct_but_no_default<T>(_: &T)
    where
        T: Sync
            + Send
            + Copy
            + Clone
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    // like `is_small_value_struct`, but too big to be copied around
    pub(crate) fn is_big_value_struct<T>(_: &T)
    where
        T: Sync
            + Send
            + Clone
            + Default
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    // like `is_big_value_struct`, but too big to be copied around, and no meaningful default
    pub(crate) fn is_big_value_struct_but_no_default<T>(_: &T)
    where
        T: Sync
            + Send
            + Clone
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    /// have everything a value type should have
    /// short of `Default` compares to `is_small_value_struct`
    pub(crate) fn is_small_value_enum<T>(_: &T)
    where
        T: Sync
            + Send
            + Copy
            + Clone
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    // like `is_small_value_enum`, but too big to be copied around
    pub(crate) fn is_big_value_enum<T>(_: &T)
    where
        T: Sync
            + Send
            + Clone
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    pub(crate) fn is_default_debug<T>(_: &T)
    where
        T: Default + std::fmt::Debug + Sync + Send,
    {
    }
    pub(crate) fn is_debug<T>(v: &T) -> usize
    where
        T: std::fmt::Debug + Sync + Send,
    {
        format!("{:?}", v).len()
    }
    pub(crate) fn is_display<T>(_: &T)
    where
        T: std::fmt::Display,
    {
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
        struct SmallValueStructButNoDefault {}

        #[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
        struct BigValueStruct {}

        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
        struct BigValueStructButNoDefault {}

        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
        enum SmallValueEnum {
            A,
        }

        #[derive(Debug, Default)]
        struct DefaultDebugStruct {}

        #[derive(Debug)]
        struct DebugStruct {}

        #[test]
        fn test_traits() {
            is_small_value_struct_but_no_default(&SmallValueStructButNoDefault {});
            is_big_value_struct(&BigValueStruct {});
            is_big_value_struct_but_no_default(&BigValueStructButNoDefault {});
            is_small_value_enum(&SmallValueEnum::A);
            is_default_debug(&DefaultDebugStruct {});
            is_debug(&DebugStruct {});
        }
    }
}
