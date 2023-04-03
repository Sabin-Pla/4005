use std::fmt::{Display, Formatter, Result};
use crate::simulation::{Duration, TimeStamp};

// components have an inspection duration
#[derive(Copy, Clone, Debug)]
pub enum Component {
    C1(Duration, Option<TimeStamp>, Option<TimeStamp>),
    C2(Duration, Option<TimeStamp>, Option<TimeStamp>),
    C3(Duration, Option<TimeStamp>, Option<TimeStamp>)
}

impl Component {
    fn mut_fields(&mut self) -> (Duration, &mut Option<TimeStamp>, &mut Option<TimeStamp>) {
        match self {
            Self::C1(dur,  start,  end) => (*dur, start, end), 
            Self::C2(dur,  start,  end) => (*dur, start, end), 
            Self::C3(dur,  start,  end) => (*dur, start, end), 
        }
    }

    fn fields(&self) -> (Duration, Option<TimeStamp>, Option<TimeStamp>) {
        match self {
            Self::C1(dur,  start,  end) => (*dur, *start, *end), 
            Self::C2(dur,  start,  end) => (*dur, *start, *end), 
            Self::C3(dur,  start,  end) => (*dur, *start, *end), 
        }
    }

    pub fn duration(&mut self) -> Duration {
        self.fields().0
    }

    pub fn inspection_start_time(&mut self) -> TimeStamp {
        self.fields().1
            .expect(format!("inspection start time called on unstarted {}", self.name()).as_str())
        
    }

    pub fn inspection_end_time(&self) -> TimeStamp {
        self.fields().2
            .expect(format!("inspection end time called on unfinished {}", self.name()).as_str())
    }

    pub fn start_inspecting(&mut self, ts: TimeStamp) {
        (*self.mut_fields().1) = Some(ts);
    }

    pub fn finish_inspecting(&mut self, now: TimeStamp) {
        let f = self.mut_fields();
        assert!(matches!(*f.2, None), "Component already finished."); 
        let dif = f.1.unwrap() + f.0 - now;
        assert!(dif.as_minutes() <=  1000.0 * f64::EPSILON, 
            "{} floating point arithmetic error threshold exceeded", dif.as_minutes()
        );
        *f.2 = Some(now);
    }

    pub fn name(&self) -> &str{
        match self {
            Self::C1(..) => "C1",
            Self::C2(..) => "C2",
            Self::C3(..) => "C3"
        }
    }

    pub fn matches(&self, other: &Component) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub fn is_finished(&self) -> bool {
        self.fields().2.is_some()
    }
}

impl Display for Component {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.name())
    }
}

impl PartialEq<Component> for Component {
    fn eq(&self, other: &Component) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}