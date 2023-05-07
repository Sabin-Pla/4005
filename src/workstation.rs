use std::collections::VecDeque;
use std::fmt::{Display, Formatter, Result};

use crate::component::Component;
use crate::event::EnqueueResult;
use crate::event::FacilityEvent;
use crate::product::Product;
use crate::simulation::Duration;
use crate::simulation::SimulationActor;
use crate::TimeStamp;

type Buffer = [Option<Component>; 2];

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let print_buffer = |buf: &Buffer| {
            if let Some(component) = buf[1] {
                return format!("[{}, {}]", buf[0].unwrap(), component);
            } else if let Some(component) = buf[0] {
                return format!("[{}]", component);
            }
            format!("[]")
        };
        match self {
            Self::W1(buf) => write!(f, "{}", print_buffer(buf)),
            Self::W2(buf1, buf2) => write!(f, "{} {}", print_buffer(buf1), print_buffer(buf2)),
            Self::W3(buf1, buf2) => write!(f, "{} {}", print_buffer(buf1), print_buffer(buf2)),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Type {
    W1(Buffer),         // P1
    W2(Buffer, Buffer), // P2
    W3(Buffer, Buffer), // P3
}

impl PartialEq<Type> for Type {
    fn eq(&self, other: &Type) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Type {
    pub fn can_work(&self) -> bool {
        match self {
            Self::W1(buf) => matches!(buf[0], Some(_)),
            Self::W2(buf1, buf2) => matches!(buf1[0], Some(_)) && matches!(buf2[0], Some(_)),
            Self::W3(buf1, buf2) => matches!(buf1[0], Some(_)) && matches!(buf2[0], Some(_)),
        }
    }

    pub fn contains(&self, component: Component) -> bool {
        let in_buf = |buf: &Buffer, component: Component| -> bool {
            buf[0] == Some(component) || buf[1] == Some(component)
        };
        match self {
            Self::W1(buf) => in_buf(buf, component),
            Self::W2(buf1, buf2) => in_buf(buf1, component) || in_buf(buf2, component),
            Self::W3(buf1, buf2) => in_buf(buf1, component) || in_buf(buf2, component),
        }
    }

    fn present_count(buf: &Buffer) -> usize {
        let count_slot = |i| match buf[i] {
            Some(_) => 1,
            None => 0,
        };
        count_slot(0) + count_slot(1)
    }

    pub fn c1_in_waiting(&self) -> usize {
        match self {
            Self::W1(buf) => Self::present_count(&buf),
            Self::W2(buf1, _) => Self::present_count(&buf1),
            Self::W3(buf1, _) => Self::present_count(&buf1),
        }
    }

    pub fn matching_count(&self, component: Component) -> usize {
        match component {
            Component::C1(..) => self.c1_in_waiting(),
            _ => match self {
                Self::W1(_) => panic!("{self} does not make {component}"),
                Self::W2(_, buf2) => Self::present_count(buf2),
                Self::W3(_, buf2) => Self::present_count(buf2),
            },
        }
    }
    pub fn first_enqueue_time(&self) -> Option<TimeStamp> {
        let c_min = |cf: Option<Component>, cs: Option<Component>| -> Option<Component> {
            if cf.is_none() && cs.is_none() {
                return None;
            } else if cf.is_none() {
                return cs;
            } else if cs.is_none() {
                return cf;
            }
            match matches!(
                cf.unwrap()
                    .inspection_start_time()
                    .partial_cmp(&cs.unwrap().inspection_start_time()),
                Some(std::cmp::Ordering::Greater)
            ) {
                true => cs,
                false => cf,
            }
        };

        let buf_min =
            |buf1: Buffer, buf2: Buffer| c_min(c_min(buf1[0], buf1[1]), c_min(buf2[0], buf2[1]));

        let c = match self {
            Self::W1(buf1) => c_min(buf1[0], buf1[1]),
            Self::W2(buf1, buf2) => buf_min(*buf1, *buf2),
            Self::W3(buf1, buf2) => buf_min(*buf1, *buf2),
        };

        match c {
            Some(c) => Some(c.inspection_start_time()),
            None => None,
        }
    }

    pub fn name(&self) -> String {
        let id = match self {
            Self::W1(_) => "1",
            Self::W2(..) => "2",
            Self::W3(..) => "3",
        };
        format!("WS{id}")
    }
}

#[derive(Clone, Debug)]
pub struct Workstation {
    assembly_durations: VecDeque<Duration>,
    current_duration: Option<(TimeStamp, Duration)>, // (start time, duration)
    ws_type: Type,
    pub products: Vec<Product>,
    pub buffer_states: Vec<(TimeStamp, Type)>,
}

impl Display for Workstation {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.current_duration {
            Some((ts, duration)) => write!(
                f,
                "{} {}\t| started: {} | duration {}",
                self.name(),
                self.ws_type,
                ts,
                duration
            ),
            None => write!(f, "{} {}", self.name(), self.ws_type),
        }
    }
}

impl Workstation {
    pub fn new(ws_type: Type, assembly_durations: VecDeque<Duration>) -> Workstation {
        Workstation {
            assembly_durations,
            ws_type,
            current_duration: None,
            products: vec![],
            buffer_states: vec![(TimeStamp::start(), ws_type)]
        }
    }

    pub fn name(&self) -> String {
        self.ws_type.name()
    }

    pub fn is_working(&self) -> bool {
        matches!(self.current_duration, Some(_))
    }

    pub fn matching_count(&self, component: Component) -> usize {
        self.ws_type.matching_count(component)
    }

    fn assemble(&mut self, timestamp: TimeStamp) -> Product {
        // consumes Components in buffer to create Product
        assert!(
            self.ws_type.can_work(),
            "assemble called but WS could not work"
        );

        let take_first_avail = |buf: &mut Buffer| -> Component {
            match buf[1] {
                Some(c) => {
                    let comp = c;
                    buf[1] = None;
                    comp
                }
                None => {
                    let comp = buf[0].unwrap();
                    buf[0] = None;
                    comp
                }
            }
        };

        match &mut self.ws_type {
            Type::W1(buf) => {
                let component = take_first_avail(buf);
                Product::from(component, None, timestamp)
            }
            Type::W2(buf1, buf2) => {
                let first = take_first_avail(buf1);
                let second = take_first_avail(buf2);
                Product::from(first, Some(second), timestamp)
            }
            Type::W3(buf1, buf2) => {
                let first = take_first_avail(buf1);
                let second = take_first_avail(buf2);
                Product::from(first, Some(second), timestamp)
            }
        }
    }

    pub fn c1_in_waiting(&self) -> usize {
        self.ws_type.c1_in_waiting()
    }

    pub fn enqueue(&mut self, ins1: bool, c: Component, now: TimeStamp) -> EnqueueResult {
        assert!(c.is_finished(), "{} {}", ins1, c);

        let add_to_buffer = |buf: &mut Buffer, mut c: Component| {
            // put c in next available slot in buffer
            // buffer cannot be full
            c.set_enqueued(now);
            if !buf[1].is_none() {
                return (false, *buf);
            }
            match buf[0] {
                Some(_) => buf[1] = Some(c),
                None => buf[0] = Some(c),
            };
            (true, *buf)
        };

        let decide_buffer = |buf_c1: &mut Buffer, other_buffer: &mut Buffer, c| match c {
            Component::C1(..) => add_to_buffer(buf_c1, c),
            Component::C2(..) => add_to_buffer(other_buffer, c),
            Component::C3(..) => add_to_buffer(other_buffer, c),
        };

        let (result, _) = match &mut self.ws_type {
            Type::W1(buf_c1) => add_to_buffer(buf_c1, c),
            Type::W2(buf_c1, buf_c2) => decide_buffer(buf_c1, buf_c2, c),
            Type::W3(buf_c1, buf_c3) => decide_buffer(buf_c1, buf_c3, c),
        };
        match result {
            true => {
                self.buffer_states.push((now, self.ws_type));
                EnqueueResult::CouldEnqueue(ins1, c, self.ws_type, now, self.is_working())
            }
            false => EnqueueResult::Fail,
        }
    }

    fn start(&mut self, start_time: TimeStamp) {
        assert!(
            matches!(self.current_duration, None),
            "WS {} which is already working was started",
            self.name()
        );
        let duration = self.assembly_durations.pop_front().expect(
            format!(
                "WS was started but \
            has no remaining duration {}",
                self.products.len()
            )
            .as_str(),
        );
        self.current_duration = Some((start_time, duration));
    }
}

impl SimulationActor for Workstation {
    fn respond_to(&mut self, event: FacilityEvent) -> Option<FacilityEvent> {
        match event {
            FacilityEvent::WorkstationStarted(ws, start_time) => {
                if &self.ws_type == &ws {
                    self.start(start_time);
                }
                None
            }
            _ => None,
        }
    }

    fn respond(&mut self, now: TimeStamp) -> Option<FacilityEvent> {
        // time until done should be zero, given margin of error for f64
        let time_until_done = self.duration_until_next_event(now).expect(
            format!(
                "WS {} called with respond(now, duration) but isn't marked \
                as working (has no self.current_duration)",
                self.name().as_str()
            )
            .as_str(),
        );
        assert!(time_until_done.as_minutes() <= 1000.0 * f64::EPSILON);

        self.current_duration = None;
        let product = self.assemble(now);
        let assembly_event = FacilityEvent::Assembled(product, self.ws_type);

        self.products.push(product);
        self.buffer_states.push((now, self.ws_type));

        // start working on the next product if it can
        if self.ws_type.can_work() {
            self.start(now);
        }

        Some(assembly_event)
    }

    fn duration_until_next_event(&self, now: TimeStamp) -> Option<Duration> {
        match self.current_duration {
            Some((start_time, duration)) => Some(start_time + duration - now),
            None => None,
        }
    }
}
