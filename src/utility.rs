#![allow(dead_code)]

#[cfg(feature = "instr_tracing")]
pub(crate) mod tracing {}

#[cfg(feature = "instr_timing")]
pub(crate) mod timing {
    use smol_str::SmolStr;

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
    /// `collect_pt` must be called first. Failing to do that, there'll be an
    /// assertion failure inside `drop`
    #[derive(Debug, Default)]
    struct Points {
        pt: Vec<f32>,
        pt_agg: Vec<f32>,
    }

    impl Points {
        fn new() -> Self {
            Self {
                pt: Vec::with_capacity(5_000),
                pt_agg: Vec::with_capacity(1_000_000),
            }
        }
        /// add a data point
        ///
        /// if `pt` threshold reaches, then data in `pt` will be collected
        /// and "moved" to `pt_agg`,
        fn add(&mut self, u: f32) {
            self.pt.push(u);
            if self.pt.len() > 5_000 {
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
    }

    /// Checking all data in `pt` has been collected
    impl Drop for Points {
        fn drop(&mut self) {
            assert!(self.pt.is_empty());
        }
    }

    struct Stat {
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
                c.insert(SmolStr::from(n), Points::default());
            }
            Self { c }
        }
        /// reset collector for reuse
        fn reset(&mut self) {
            self.c.values_mut().for_each(|v| {
                assert!(v.pt.is_empty());
                v.pt.clear();
                v.pt_agg.clear();
            });
        }
        // `name` is the instruction name given when `new` is called
        pub(crate) fn start(&mut self, name: SmolStr) -> InstructionTiming {
            assert!(self.c.contains_key(&name));
            InstructionTiming {
                c: self,
                a: std::time::Instant::now(),
                name,
            }
        }
        const fn baseline() -> &'static str {
            "BASELINE"
        }
        /// this must be called before calling `fmt!` to collection final analyzed data,
        /// it clears all data points afterwards
        pub(crate) fn finalize<F>(&mut self, f: F)
        where
            F: Fn(String),
        {
            (0..5_000_000).for_each(|_| {
                let _a = self.start(SmolStr::from(Self::baseline()));
            });
            self.c.values_mut().for_each(Points::collect_pt);
            f(self.collect());
            self.reset();
        }
        fn points_to_stat(v: &[f32]) -> Option<Stat> {
            if v.is_empty() {
                return None;
            }

            let (min, max) = (
                v.iter().map(|&e| e as u128).min(),
                v.iter().map(|&e| e as u128).max(),
            );
            let mean = (!v.is_empty()).then_some(v.iter().sum::<f32>() / v.len() as f32);
            let std_deviation = mean.map(|m| {
                let variance = v
                    .iter()
                    .map(|e| {
                        let diff = m - *e;
                        diff * diff
                    })
                    .sum::<f32>()
                    / v.len() as f32;
                variance.sqrt()
            });
            Some(Stat {
                mean: mean.unwrap(),
                min: min.unwrap(),
                max: max.unwrap(),
                std_deviation: std_deviation.unwrap(),
            })
        }
        fn stat_to_string(name: &SmolStr, points: &[f32]) -> Option<String> {
            if let Some(Stat {
                mean,
                min,
                max,
                std_deviation,
            }) = Self::points_to_stat(points)
            {
                let cv = std_deviation / mean;
                let cv_s = {
                    let cv_s = format!("{:6.2}", cv);
                    if cv > 1.0 && console::Term::stdout().features().colors_supported() {
                        console::style(cv_s).red().to_string()
                    } else {
                        cv_s
                    }
                };
                Some(format!(
                    "{:10}: mean ={:10.2} std ={:10.2} cv ={} | min ={:7}, max ={:7}\n",
                    name, mean, std_deviation, cv_s, min, max
                ))
            } else {
                None
            }
        }
        fn collect(&self) -> String {
            self.c
                .iter()
                .map(|(k, v)| Self::stat_to_string(k, &v.pt_agg))
                .filter(Option::is_some)
                .flatten()
                .collect()
        }
    }

    impl Drop for InstructionTimingCollector {
        /// to ensure all data points have been dropped incase `finalize` not gets called
        ///
        /// without this, assertion in `Points::drop` will fail
        fn drop(&mut self) {
            self.reset();
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
                .add(self.a.elapsed().as_nanos() as f32);
        }
    }
}

#[cfg(test)]
pub(crate) mod traits {
    /// have everything a value type should have
    pub(crate) fn is_small_value_struct<T>(_: &T)
    where
        T: Sync
            + Send
            + Copy
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
    pub(crate) fn is_big_but_incomparable_and_no_default<T>(_: &T)
    where
        T: std::fmt::Debug + Sync + Send + Clone,
    {
    }
    pub(crate) fn is_big_but_incomparable<T>(_: &T)
    where
        T: Default + std::fmt::Debug + Sync + Send + Clone,
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
}
