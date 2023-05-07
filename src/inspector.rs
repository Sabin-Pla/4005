use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter, Result};
use std::rc::Rc;

use crate::component::Component;
use crate::event::{EnqueueResult, FacilityEvent};
use crate::random::Random;
use crate::simulation::SimulationActor;
use crate::simulation::TimeStamp;
use crate::workstation::Workstation;
use crate::Duration;
use crate::Product;

pub trait Inspector {
    fn inspect_next(&mut self, now: TimeStamp) -> Option<Component>;
    fn dispatch_component(&mut self, i: usize, now: TimeStamp) -> EnqueueResult;
    fn is_1(&self) -> bool;
    fn produces(&self, product: Product) -> bool {
        match self.is_1() {
            true => matches!(product, Product::P1(..)),
            false => match product {
                Product::P1(..) => false,
                Product::P2(..) => true,
                Product::P3(..) => true,
            },
        }
    }

    fn held_components(&self, finished_only: bool) -> Vec<usize>;
    fn is_blocked(&self) -> bool;
    fn set_unblocked(&mut self, now: TimeStamp);
    fn set_blocked(&mut self, now: TimeStamp);
    fn next_end_time(&self) -> Option<TimeStamp>;

    fn name(&self) -> &str {
        match self.is_1() {
            true => "Inspector1",
            false => "Inspector2",
        }
    }

    fn place_routine(&mut self, now: TimeStamp, expect_blocked: bool) -> Option<FacilityEvent> {
        if expect_blocked {
            assert!(self.is_blocked());
        } else {
            assert!(!self.is_blocked());
        }

        for i in self.held_components(true) {
            match self.dispatch_component(i, now) {
                EnqueueResult::CouldEnqueue(_, component, ws, ts, ws_is_working) => {
                    log!(
                        "Enqueue {} {} {} {} {}",
                        self.is_1(),
                        component,
                        ws.name(),
                        ws.can_work(),
                        ws_is_working
                    );
                    assert!(ws.contains(component));
                    self.remove_component(i);
                    if expect_blocked {
                        self.set_unblocked(now);
                    }
                    self.inspect_next(now);
                    return match !ws_is_working && ws.can_work() {
                        false => None, // ws restarted itself
                        true => Some(FacilityEvent::WorkstationStarted(ws, ts)),
                    };
                }
                EnqueueResult::Fail => continue,
            };
        }

        if !self.is_1() && self.held_components(false).len() < 2 {
            self.inspect_next(now);
            self.set_unblocked(now);
            return None;
        }

        if !expect_blocked {
            self.set_blocked(now);
        }
        None
    }

    // returns the list of timestamps block set_blocked and set_unblocked
    // were called
    fn blocked_times(&self) -> &Vec<TimeStamp>;
    fn remove_component(&mut self, i: usize);
    fn finish_inspection(&mut self, now: TimeStamp);
    fn working_on(&self) -> String;
    fn inspection_times(&mut self) -> (&mut VecDeque<TimeStamp>, &mut VecDeque<TimeStamp>);
    fn log_departure(&mut self, now: TimeStamp);
}

pub struct Inspector1 {
    ws: [Rc<RefCell<Workstation>>; 3],
    durations_c1: VecDeque<Duration>,
    held_component: Option<Component>,
    next_finish_time: Option<TimeStamp>,
    is_blocked: bool,
    // logs each time a block operation is called
    blocked_times: Vec<TimeStamp>,
    inspection_times: VecDeque<TimeStamp>,
    departure_times: VecDeque<TimeStamp>,
}

impl Inspector1 {
    pub fn new(ws: [Rc<RefCell<Workstation>>; 3], durations_c1: VecDeque<Duration>) -> Self {
        Inspector1 {
            ws,
            durations_c1,
            held_component: None,
            next_finish_time: None,
            is_blocked: true,
            blocked_times: vec![],
            inspection_times: vec![].into(),
            departure_times: vec![].into(),
        }
    }
}

impl Inspector for Inspector1 {
    fn held_components(&self, finished_only: bool) -> Vec<usize> {
        match self.held_component {
            Some(c) => vec![0],
            None => vec![],
        }
    }

    fn is_1(&self) -> bool {
        true
    }

    fn is_blocked(&self) -> bool {
        self.is_blocked
    }

    fn blocked_times(&self) -> &Vec<TimeStamp> {
        &self.blocked_times
    }

    fn working_on(&self) -> String {
        match self.held_component {
            Some(c1) => "C1".to_string(),
            None => "".to_string(),
        }
    }

    fn dispatch_component(&mut self, i: usize, now: TimeStamp) -> EnqueueResult {
        // attempts to move the component at index i into a workstation
        assert!(i == 0); // there is only 1 index
        let c = self
            .held_component
            .expect("dispatch called but there is no component");
        assert!(c.is_finished());
        assert!(matches!(c, Component::C1(..)));

        let mut ws1 = self.ws[0].borrow_mut();
        let mut ws2 = self.ws[1].borrow_mut();
        let mut ws3 = self.ws[2].borrow_mut();
        let awaiting = [
            ws1.c1_in_waiting(),
            ws2.c1_in_waiting(),
            ws3.c1_in_waiting(),
        ];
        const USE_NEW_STRATEGY: bool = true; 
        match USE_NEW_STRATEGY {
            true => {
                if awaiting[0] < awaiting[1] && awaiting[0] < awaiting[2] {
                    return ws1.enqueue(true, c, now);
                } else if awaiting[1] <= awaiting[2] && awaiting[1] <= awaiting[0] {
                    return ws2.enqueue(true, c, now);
                } else if awaiting[2] <= awaiting[0] && awaiting[2] <= awaiting[1] {
                    return ws3.enqueue(true, c, now);
                } else {
                    panic!("Bad branch")
                }
            },
            false => {
                if awaiting[2] < awaiting[1] && awaiting[2] < awaiting[0] {
                    return ws3.enqueue(true, c, now);
                } else if awaiting[1] <= awaiting[2] && awaiting[1] < awaiting[0] {
                    return ws2.enqueue(true, c, now);
                } else if awaiting[0] <= awaiting[1] && awaiting[0] <= awaiting[2] {
                    return ws1.enqueue(true, c, now);
                } else {
                    panic!("Bad branch")
                }
            }
        }
    }

    fn inspect_next(&mut self, now: TimeStamp) -> Option<Component> {
        assert!(!self.is_blocked());
        match self.durations_c1.pop_front() {
            Some(duration) => {
                self.next_finish_time = Some(now + duration);
                let mut component = Component::new(duration, 1);
                component.start_inspecting(now);
                let component = Some(component);
                self.held_component = component;
                self.inspection_times.push_back(now);
                component
            }
            None => {
                self.next_finish_time = None;
                None
            }
        }
    }

    fn next_end_time(&self) -> Option<TimeStamp> {
        self.next_finish_time
    }

    fn set_unblocked(&mut self, now: TimeStamp) {
        assert!(self.is_blocked);
        self.blocked_times.push(now);
        self.is_blocked = false;
    }

    fn set_blocked(&mut self, now: TimeStamp) {
        assert!(!self.is_blocked);
        self.blocked_times.push(now);
        self.is_blocked = true;
    }

    fn remove_component(&mut self, i: usize) {
        assert!(i == 0);
        assert!(matches!(self.held_component, Some(_)));
        self.held_component = None;
    }

    fn finish_inspection(&mut self, now: TimeStamp) {
        let mut c = self
            .held_component
            .expect("no ins1 component to finish inspecting");
        c.finish_inspecting(now);
        self.held_component = Some(c);
    }

    fn log_departure(&mut self, now: TimeStamp) {
        self.departure_times.push_back(now);
    }

    fn inspection_times(&mut self) -> (&mut VecDeque<TimeStamp>, &mut VecDeque<TimeStamp>) {
        (&mut self.inspection_times, &mut self.departure_times)
    }
}

pub struct Inspector2 {
    ws: [Rc<RefCell<Workstation>>; 2],
    durations_c2: VecDeque<Duration>,
    durations_c3: VecDeque<Duration>,
    held_c2: Option<Component>,
    held_c3: Option<Component>,
    next_finish_time: Option<(Component, TimeStamp)>,
    is_blocked: bool,
    random: Random,
    blocked_times: Vec<TimeStamp>,
    inspection_times: VecDeque<TimeStamp>,
    departure_times: VecDeque<TimeStamp>,
}

impl Inspector2 {
    pub fn new(
        ws: [Rc<RefCell<Workstation>>; 2],
        durations_c2: VecDeque<Duration>,
        durations_c3: VecDeque<Duration>,
    ) -> Self {
        Inspector2 {
            ws,
            durations_c2,
            durations_c3,
            held_c2: None,
            held_c3: None,
            next_finish_time: None,
            is_blocked: true,
            random: Random::new(),
            blocked_times: vec![],
            inspection_times: vec![].into(),
            departure_times: vec![].into(),
        }
    }

    fn decide_next_component(&mut self) -> Option<Component> {
        // decides the next component.
        // first pick from the only available set of components (if there is only 1).
        // If both sets are available, start working on whatever is blocked.
        // If neither case is true, pick a component at random.
        if self.durations_c2.len() == 0 && self.durations_c3.len() != 0 {
            return Some(Component::new(self.durations_c3.pop_front().unwrap(), 3));
        } else if self.durations_c2.len() != 0 && self.durations_c3.len() == 0 {
            return Some(Component::new(self.durations_c2.pop_front().unwrap(), 2));
        } else if self.durations_c2.len() != 0 && self.durations_c3.len() != 0 {
            let c2_count = self.ws[0]
                .borrow()
                .matching_count(Component::new(Duration::never(), 2));
            let c3_count = self.ws[1]
                .borrow()
                .matching_count(Component::new(Duration::never(), 3));

            if c2_count == 2 && c3_count != 2 && !self.held_c3.is_some() {
                // c2 is full so work on c3
                return Some(Component::new(self.durations_c3.pop_front().unwrap(), 3));
            } else if c3_count == 2 && c2_count != 2 && !self.held_c2.is_some() {
                return Some(
                    // c3 is full so work on c2
                    Component::new(self.durations_c2.pop_front().unwrap(), 2),
                );
            } else if c3_count == 2 && c2_count == 2 {
                return None; // blocked
            }
            return match self.random.boolean() {
                true => Some(Component::new(self.durations_c3.pop_front().unwrap(), 3)),
                false => Some(Component::new(self.durations_c2.pop_front().unwrap(), 2)),
            };
        }
        None
    }
}

impl Inspector for Inspector2 {
    fn is_1(&self) -> bool {
        false
    }

    fn held_components(&self, finished_only: bool) -> Vec<usize> {
        let mut v = vec![];

        if self.held_c2.is_some()
            && ((finished_only && self.held_c2.unwrap().is_finished()) || !finished_only)
        {
            v.push(2);
        }
        if self.held_c3.is_some()
            && ((finished_only && self.held_c3.unwrap().is_finished()) || !finished_only)
        {
            v.push(3);
        }
        v
    }

    fn blocked_times(&self) -> &Vec<TimeStamp> {
        &self.blocked_times
    }

    fn dispatch_component(&mut self, i: usize, now: TimeStamp) -> EnqueueResult {
        let c = match i {
            2 => self.held_c2.unwrap(),
            3 => self.held_c3.unwrap(),
            _ => panic!(),
        };
        match c.is_finished() {
            true => match c {
                Component::C2(..) => self.ws[0].borrow_mut().enqueue(false, c, now),
                Component::C3(..) => self.ws[1].borrow_mut().enqueue(false, c, now),
                Component::C1(..) => panic!("Inspector 2 does not inspect Component 1"),
            },
            false => EnqueueResult::Fail,
        }
    }

    fn inspect_next(&mut self, now: TimeStamp) -> Option<Component> {
        match self.decide_next_component() {
            Some(mut component) => {
                component.start_inspecting(now);
                self.next_finish_time = Some((component, now + component.duration()));
                match component {
                    Component::C1(..) => panic!(),
                    Component::C2(..) => {
                        assert!(matches!(self.held_c2, None));
                        self.held_c2 = Some(component);
                    }
                    Component::C3(..) => {
                        assert!(matches!(self.held_c3, None));
                        self.held_c3 = Some(component);
                    }
                }
                self.inspection_times.push_back(now);
                Some(component)
            }
            None => {
                self.set_blocked(now);
                self.next_finish_time = None;
                None
            }
        }
    }

    fn next_end_time(&self) -> Option<TimeStamp> {
        match self.next_finish_time {
            Some((_c, ts)) => Some(ts),
            None => None,
        }
    }

    fn is_blocked(&self) -> bool {
        self.is_blocked
    }

    fn set_unblocked(&mut self, now: TimeStamp) {
        assert!(self.is_blocked);
        self.blocked_times.push(now);
        self.is_blocked = false;
    }

    fn set_blocked(&mut self, now: TimeStamp) {
        assert!(!self.is_blocked);
        self.blocked_times.push(now);
        self.is_blocked = true;
    }

    fn remove_component(&mut self, i: usize) {
        match i {
            2 => self.held_c2 = None,
            3 => self.held_c3 = None,
            _ => panic!(),
        }
    }

    fn finish_inspection(&mut self, now: TimeStamp) {
        if let Some(mut c2) = self.held_c2 {
            if !c2.is_finished() {
                c2.finish_inspecting(now);
                assert!(matches!(c2, Component::C2(..)));
                self.held_c2 = Some(c2);
            }
        } else if let Some(mut c3) = self.held_c3 {
            if !c3.is_finished() {
                c3.finish_inspecting(now);
                assert!(matches!(c3, Component::C3(..)));
                self.held_c3 = Some(c3);
            }
        }
    }

    fn working_on(&self) -> String {
        // gets a string representing what
        // inspector 2 is working on. i.e, [, C3 (in progress)]
        // one element is always in progress and one is always done
        // (or not present)

        let status = |done: bool| -> &str {
            match done {
                false => "(in progress)",
                true => "(done)",
            }
        };

        let s1 = match self.held_c2 {
            Some(c1) => format!("C2 {}", status(c1.is_finished())),
            None => "".to_string(),
        };

        let s2 = match self.held_c3 {
            Some(c1) => format!("C3 {}", status(c1.is_finished())),
            None => "".to_string(),
        };

        format!("[{}, {}]", s1, s2)
    }

    fn log_departure(&mut self, now: TimeStamp) {
        self.departure_times.push_back(now);
    }

    fn inspection_times(&mut self) -> (&mut VecDeque<TimeStamp>, &mut VecDeque<TimeStamp>) {
        (&mut self.inspection_times, &mut self.departure_times)
    }
}

impl SimulationActor for &mut dyn Inspector {
    fn respond_to(&mut self, event: FacilityEvent) -> Option<FacilityEvent> {
        match event {
            // if a workstation assembled a component
            // then the inspector may no longer be blocked
            FacilityEvent::Assembled(product, _ws) => {
                if self.produces(product) {
                    self.log_departure(product.timestamp());
                }

                match self.produces(product) && self.is_blocked() {
                    true => self.place_routine(product.timestamp(), true),
                    false => None, // Not this inspector
                }
            }
            FacilityEvent::SimulationStarted => {
                self.set_unblocked(event.timestamp());
                self.inspect_next(event.timestamp()).expect(
                    format!("Failure loading inspection times for {}", self.name()).as_str(),
                );
                None
            }
            _ => None,
        }
    }

    fn respond(&mut self, now: TimeStamp) -> Option<FacilityEvent> {
        // is called when inspector finishes, but never to unblock the inspector
        // unblocking is done through respond_to(FacilityEvent::Assembled)
        assert!(!self.is_blocked());
        self.finish_inspection(now);
        self.place_routine(now, false)
    }

    fn duration_until_next_event(&self, now: TimeStamp) -> Option<Duration> {
        match self.is_blocked() {
            true => None,
            false => match self.next_end_time() {
                Some(ts) => Some(ts - now),
                None => None,
            },
        }
    }
}

impl Display for &mut dyn Inspector {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{} | blocked: {} | holding {} | in queue: {}",
            self.name(),
            self.is_blocked(),
            self.held_components(false).len(),
            self.working_on()
        )
    }
}
