/// Linear Congruential Generator
///
/// Xi+1 = (a x Xi + c) mod m
///
/// here state is Xi, and m is max value of the data type
pub struct Lcg {
    a: u8,
    c: u8,
    state: u8,
}

impl Lcg {
    pub fn new(a: u8, c: u8, state: u8) -> Lcg {
        Lcg { a, c, state }
    }

    pub fn next(&mut self) -> u8 {
        self.state = self.state.wrapping_mul(self.a).wrapping_add(self.c);
        self.state
    }
}
