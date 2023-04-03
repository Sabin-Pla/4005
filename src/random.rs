
use std::time::{SystemTime, UNIX_EPOCH};


pub struct Random {
    gen: LcmGenerator
}

impl Random {
    pub fn new() -> Self {
        Random { gen: LcmGenerator::new() }
    }

    pub fn boolean(&mut self) -> bool {
        self.gen.set_next() % 2 == 0
    }

    pub fn float(&mut self) -> f64 {
        self.gen.set_next() as f64 / self.gen.m as f64
    }
}

struct LcmGenerator {
    a: u32, // multiplier
    c: u64, // increment
    pub m: u64, // modulus
    x: u64
}

impl LcmGenerator {

    const INIT_SEED: u32 = 11774353;
    const BIG_PRIME: u64 = 999999000001;
        // larger primes that can be represented exist. I.e,
        //           3318308475676071413, 
        //           18446744073709551615
        //  (u64 max 10888869450418352160768000001)
        // amount of shift in init c value 
        // (default 25) is tuned experimentally
        // hard to find a good number for larger primes

    pub fn new() -> Self {
        let seed: u32 = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(dur) => {
                dur.subsec_nanos().reverse_bits()
            },
            Err(_) => { 
                panic!("SystemTime before UNIX EPOCH!"); 
            }
        };
        Self::with_seed(seed)
    }

    pub fn with_seed(seed: u32) -> Self  {
        log!("seed:  {seed}");
        let seed = Self::INIT_SEED as u64 + seed as u64;
        let b = 40;
        let m = 2u64.pow(b);
        log!("m:  {m}");

        let mut c = seed << 25;
        while c < Self::INIT_SEED as u64 || !relatively_prime(m, c) {
            c = Self::next(
                    c + c >> 5, 
                    (4 * ((seed>>7) + 1) + 1) as u32, 
                    0, 
                    Self::BIG_PRIME);
            log!("c:  {c}");
        }

        let mut a = Self::next(seed + 1,
            Self::INIT_SEED, c, 2u64.pow(24)) as u32;
        while a < Self::INIT_SEED || !(gcd(a as u64, 4) == 4) {
            a = Self::next(a as u64, 
                    Self::INIT_SEED, c, 
                    2u64.pow(24)) as u32;
            log!("a:  {a}");
        }
        a += 1; // since gcd(a, 4) == 4 earlier, a = 4k+1
        assert!((a-1) % 4 == 0);
        log!("a:  {a}");

        LcmGenerator {
            a, // <= 2^24 - 1
            x: Self::next((seed << 2) as u64, a, c, m),
            c, // <= Self::BIG_PRIME - 1 
            m  // <= 2^40
        }
    }

    pub fn set_next(&mut self) -> u64 {
        self.x = ((self.a as u64) * self.x + self.c) % (self.m);
        self.x
    }

    fn next(x: u64, a: u32, c: u64, m: u64) -> u64 {
        (a as u64)
            .wrapping_mul(x)
            .wrapping_add(c)
        .wrapping_rem(m)
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    let mut t;
    while b != 0 {
        t = a;
        a = b;
        b = t.wrapping_rem(b);
    }
    a
}

fn relatively_prime(a : u64, b: u64) -> bool {
    return gcd(a, b) == 1
}
