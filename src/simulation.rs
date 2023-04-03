use std::cmp::Ordering;
use std::ops::{Add, Sub, AddAssign, Mul};
use std::fmt::{Display, Result, Formatter};

use crate::event::FacilityEvent;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct TimeStamp {
	minutes: f64
}

impl TimeStamp {
	pub fn start() -> Self {
		TimeStamp {
			minutes: 0.0
		}
	}

	pub fn get(&self) -> f64 {
		self.minutes
	} 
}

impl Display for TimeStamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:.2}", self.minutes)
    }
}

impl Add<Duration> for TimeStamp {
    type Output = TimeStamp;

    fn add(self, rhs: Duration) -> Self::Output {
    	Self { minutes: self.minutes + rhs.as_minutes() }
    }
}

impl AddAssign<Duration> for TimeStamp {
    fn add_assign(&mut self, rhs: Duration) {
    	self.minutes += rhs.as_minutes()
    }
}


impl Sub<TimeStamp> for TimeStamp {
    type Output = Duration;

    fn sub(self, rhs: TimeStamp) -> Self::Output {
    	Duration {
    		minutes: self.minutes - rhs.minutes
    	}
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Duration {
	minutes: f64
}

impl Display for Duration {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:.2}", self.minutes)
    }
}

impl Ord for Duration {
	fn cmp(&self, other: &Self) -> Ordering {
		match f64::min(self.minutes, other.minutes) == self.minutes {
			true => Ordering::Less,
			false => Ordering::Greater
		}
	}
}

impl Eq for Duration {}

impl Duration {
	pub fn from(s: String) -> Self {
		Duration { minutes: s.parse::<f64>().unwrap() }
	}

	pub fn of_minutes(m: f64) -> Self {
		Duration { minutes: m }
	}

	pub fn none() -> Self {
		Duration { minutes: 0.0 }
	}

	pub fn as_minutes(&self) -> f64 {
		self.minutes
	}

	pub fn never() -> Self {
		Duration { minutes: f64::INFINITY }
	}
}


impl Add for Duration {
    type Output = Duration;
    fn add(self, rhs: Duration) -> Self::Output {
    	Self { minutes: self.minutes + rhs.minutes }
    }
}

impl Sub for Duration {
    type Output = Duration;
    fn sub(self, rhs: Duration) -> Self::Output {
    	Self { minutes: self.minutes - rhs.minutes }
    }
}

impl Mul<f64> for Duration  {
	type Output = Duration;
	fn mul(self, rhs: f64) -> Duration  {
		Duration { minutes: self.minutes * rhs }
	}
}

impl Mul<Duration> for f64  {
	type Output = Duration;
	fn mul(self, rhs: Duration) -> Duration  {
		Duration { minutes: self * rhs.minutes }
	}
}

// a simulation actor can either respond to some
// event or just produce an event in response to time passing.
pub trait SimulationActor {
	fn respond_to(&mut self, _: FacilityEvent) -> Option<FacilityEvent>;
	fn respond(&mut self, now: TimeStamp) -> Option<FacilityEvent>;
	fn duration_until_next_event(&self, now: TimeStamp) -> Option<Duration>;
}
