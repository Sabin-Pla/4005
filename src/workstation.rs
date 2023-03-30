
use std::collections::VecDeque;

use crate::component::Component;
use crate::product::Product;
use crate::event::FacilityEvent;
use crate::TimeStamp;
use crate::simulation::SimulationActor;
use crate::simulation::Duration;
use crate::event::EnqueueResult;

type Buffer = [Option<Component>; 2];

#[derive(Copy, Clone, Debug)]
pub enum Type {
    W1(Buffer), // P1
    W2(Buffer, Buffer), // P2
    W3(Buffer, Buffer) // P3
}

#[derive(Clone, Debug)]
pub struct Workstation {
    assembly_durations: VecDeque<Duration>,
    current_duration: Option<(TimeStamp, Duration)>, // (start time, duration)
    ws_type: Type,
}

impl Workstation {

    pub fn new(ws_type: Type, assembly_durations: VecDeque<Duration>) -> Workstation {
        Workstation {
            assembly_durations,
            ws_type,
            current_duration: None
        }
    }

    pub fn name(&self) -> String {
        let id = match self.ws_type {
            Type::W1(_) => "1", 
            Type::W2(..) => "2",
            Type::W3(..) => "3"
        };
        format!("WS{id}")
    }

    fn empty_count(buf: &Buffer) -> usize {
        buf.iter().fold(0, |acc, c| {
            acc + match c {
                Some(_) => 0,
                None => 1
            }
        })
    }

    pub fn can_work(&self) -> bool {
        match self.ws_type {
            Type::W1(buf) => matches!(buf[0], None),
            Type::W2(buf1, buf2) => 
                !(matches!(buf1[0], None)  && matches!(buf2[0], None)),
            Type::W3(buf1, buf2) =>
                !(matches!(buf1[0], None)  && matches!(buf2[0], None))
        }
    }

    fn assemble(&mut self, timestamp: TimeStamp) -> Product {
        // consumes Components in buffer to create Product
        assert!(self.can_work(), "assemble called but WS could not work");

        let take_first_avail = |buf: &mut Buffer| -> Component {
            match buf[1] {
                Some(c) => {
                    let comp = buf[1].unwrap();
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

        match self.ws_type {
            Type::W1(mut buf) => {
                let component = take_first_avail(&mut buf);
                Product::from(component, None, timestamp)
            },
            Type::W2(mut buf1, mut buf2) => {
                let first = take_first_avail(&mut buf1);
                let second = take_first_avail(&mut buf2);
                Product::from(first, Some(second), timestamp)
            },
            Type::W3(mut buf1, mut buf2) => {
                let first = take_first_avail(&mut buf1);
                let second = take_first_avail(&mut buf2);
                Product::from(first, Some(second), timestamp)
            }
        }
    }

    pub fn unprocessed_components(&self) -> usize {
        match self.ws_type {
            Type::W1(mut buf) => Self::empty_count(&mut buf),
            Type::W2(mut buf1, mut buf2) => Self::empty_count(&mut buf1) + 
                Self::empty_count(&mut buf2),
            Type::W3(mut buf1, mut buf2) => Self::empty_count(&mut buf1) + 
                Self::empty_count(&mut buf2)
        }
    }

    pub fn enqueue(&mut self, ins1: bool, 
            c: Component, now: TimeStamp) -> EnqueueResult {
        let add_to_buffer = |buf: &mut Buffer, c| {
            // put c in next available slot in buffer 

            // buffer cannot be full
            if !buf[1].is_none() {
                return EnqueueResult::Fail
            }

            match buf[0] {
                Some(_) => buf[1] = Some(c),
                None => buf[0] = Some(c)
            };
            EnqueueResult::CouldEnqueue(ins1, c, self.ws_type, now)
        };

        let decide_buffer = |buf_c1: &mut  Buffer, other_buffer: &mut  Buffer, c| {
            match c {
                Component::C1(..) => add_to_buffer(buf_c1, c),
                Component::C2(..) => add_to_buffer(other_buffer, c),
                Component::C3(..) => add_to_buffer(other_buffer, c)
            }
        };

        match self.ws_type {
            Type::W1(mut buf_c1) => add_to_buffer(&mut buf_c1, c),
            Type::W2(mut buf_c1, mut buf_c2) => decide_buffer(&mut buf_c1, &mut buf_c2, c),
            Type::W3(mut buf_c1, mut buf_c3) => decide_buffer(&mut buf_c1, &mut buf_c3, c),
        }
    }
}


impl SimulationActor for Workstation {
    fn respond_to(&mut self, event: FacilityEvent) -> Vec<FacilityEvent> {
        match event {
            FacilityEvent::WorkstationStarted(ws, start_time) => {
                if matches!(&self, ws) {
                    assert!(matches!(self.current_duration, None),
                        "WS {} which is already working was started", self.name());
                    let duration = self.assembly_durations
                        .pop_front().expect("WS was started but \
                        has no remaining duration");
                    self.current_duration = Some((start_time, duration));
                }
                Vec::default()
            },
            _ => Vec::default()
        }
    }

    fn respond(&mut self, now: TimeStamp, duration: Duration) -> Vec<FacilityEvent> {

        // time until done should be zero, given margin of error for f64
        let time_until_done = self.duration_until_next_event(now).expect(
            format!("WS {} called with respond(now, duration) but isn't marked \
                as working (has no self.current_duration)", self.name().as_str()
                ).as_str()); 
        assert!(time_until_done.as_minutes() <= f64::EPSILON * 0.005);

        self.current_duration = None;
        vec!(FacilityEvent::Assembled(
            self.assemble(now), self.ws_type))
    }

    fn duration_until_next_event(&self, now: TimeStamp) -> Option<Duration> {
        match self.current_duration {
            Some((start_time, duration)) => Some(start_time + duration - now),
            None => None
        }
    }
}