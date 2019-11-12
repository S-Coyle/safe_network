// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use rand::{Rand, Rng, SeedableRng, XorShiftRng};
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

pub struct TestRng(XorShiftRng);

impl TestRng {
    #[cfg(feature = "mock_base")]
    pub fn new_rng(&mut self) -> Self {
        Self::from_seed(self.gen())
    }
}

impl Rng for TestRng {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }
}

impl SeedableRng<Seed> for TestRng {
    fn from_seed(seed: Seed) -> Self {
        Self(XorShiftRng::from_seed(seed.0))
    }

    fn reseed(&mut self, seed: Seed) {
        self.0.reseed(seed.0)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct Seed([u32; 4]);

impl Seed {
    #[cfg(feature = "mock_base")]
    pub fn from_env(name: &str) -> Option<Self> {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse().ok())
    }
}

impl FromStr for Seed {
    type Err = ParseError;

    fn from_str(mut input: &str) -> Result<Self, Self::Err> {
        let mut seed = [0u32; 4];

        skip_whitespace(&mut input);
        skip(&mut input, '[')?;

        for (index, value) in seed.iter_mut().enumerate() {
            skip_whitespace(&mut input);

            if index > 0 {
                skip(&mut input, ',')?;
                skip_whitespace(&mut input);
            }

            *value = parse_u32(&mut input)?;
        }

        skip_whitespace(&mut input);
        skip(&mut input, ']')?;

        Ok(Self(seed))
    }
}

impl Rand for Seed {
    fn rand<R: Rng>(rng: &mut R) -> Self {
        Self(rng.gen())
    }
}

impl Display for Seed {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ParseError;

fn skip_whitespace(input: &mut &str) {
    *input = input.trim_start();
}

fn skip(input: &mut &str, ch: char) -> Result<(), ParseError> {
    if input.starts_with(ch) {
        *input = &input[1..];
        Ok(())
    } else {
        Err(ParseError)
    }
}

fn parse_u32(input: &mut &str) -> Result<u32, ParseError> {
    let mut empty = true;
    let mut output = 0;

    while let Some(digit) = input.chars().next().and_then(|ch| ch.to_digit(10)) {
        empty = false;
        output = output * 10 + digit;
        *input = &input[1..];
    }

    if empty {
        Err(ParseError)
    } else {
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_seed() {
        assert_eq!("[0, 0, 0, 0]".parse(), Ok(Seed([0, 0, 0, 0])));
        assert_eq!("[0, 1, 2, 3]".parse(), Ok(Seed([0, 1, 2, 3])));
        assert_eq!(
            "[2173726344, 4077344496, 2175816672, 3385125285]".parse(),
            Ok(Seed([
                2_173_726_344,
                4_077_344_496,
                2_175_816_672,
                3_385_125_285
            ]))
        );
        assert_eq!("".parse(), Err::<Seed, _>(ParseError));
    }
}
