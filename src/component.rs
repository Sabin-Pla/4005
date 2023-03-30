
use crate::simulation::{Duration, TimeStamp};

// components have an inspection duration
#[derive(Copy, Clone, Debug)]
pub enum Component {
    C1(Duration, Option<TimeStamp>, Option<TimeStamp>),
    C2(Duration, Option<TimeStamp>, Option<TimeStamp>),
    C3(Duration, Option<TimeStamp>, Option<TimeStamp>)
}

impl Component {
    fn fields(&mut self) -> (Duration, &mut Option<TimeStamp>, &mut Option<TimeStamp>) {
        match self {
            Self::C1(dur,  start,  end) => (*dur, start, end), 
            Self::C2(dur,  start,  end) => (*dur, start, end), 
            Self::C3(dur,  start,  end) => (*dur, start, end), 
        }
    }

    pub fn duration(&mut self) -> Duration {
        self.fields().0
    }

    pub fn inspection_start_time(&mut self) -> TimeStamp {
        (*self.fields().1)
            .expect(format!("inspection start time called on unstarted {}", self.name()).as_str())
        
    }

    pub fn inspection_end_time(&mut self) -> TimeStamp {
        (*self.fields().2)
            .expect(format!("inspection end time called on unfinished {}", self.name()).as_str())
    }

    pub fn start_inspecting(&mut self, ts: TimeStamp) {
        (*self.fields().1) = Some(ts);
    }

    pub fn finish_inspecting(&mut self, now: TimeStamp) {
        let f = self.fields();
        let dif = f.1.unwrap() + f.0 - now;
        assert!(dif.as_minutes() <= f64::EPSILON * 0.005);
        *f.2 = Some(now);
        
    }

    pub fn name(&self) -> &str{
        match self {
            Self::C1(..) => "C1",
            Self::C2(..) => "C2",
            Self::C3(..) => "C3"
        }
    }
}
