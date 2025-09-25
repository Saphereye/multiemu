/// Linear Congruential Generator
///
/// Xi+1 = (a x Xi + c) mod m
///
/// here state is Xi, and m is max value of the data type
pub struct LCG {
    state: u8,
    a: u8,
    c: u8,
}

impl LCG {
    pub fn new(a: u8, c: u8, state: u8) -> LCG {
        LCG { state: state, a: a, c: c }
    }

    pub fn next(&mut self) -> u8 {
        self.state = self.state.wrapping_mul(self.a).wrapping_add(self.c);
        self.state
    }
}
