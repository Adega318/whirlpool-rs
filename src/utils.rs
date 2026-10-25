const DIGIT_LENGTH: usize = 12;

pub fn fmt_num(n: f64) -> String {
    for precision in (0..DIGIT_LENGTH).rev() {
        let s = format!("{n:.precision$}");
        if s.len() <= DIGIT_LENGTH {
            return format!("{s:>width$}", width = DIGIT_LENGTH);
        }
    }

    format!("{n:>width$.0}", width = DIGIT_LENGTH)
}
