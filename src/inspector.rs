use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;

use crate::Product;
use crate::component::Component;
use crate::Duration;
use crate::event::{EnqueueResult, FacilityEvent};
use crate::simulation::SimulationActor;
use crate::simulation::TimeStamp;
use crate::workstation::Workstation;
use crate::random::Random;


pub trait Inspector {

    fn inspect_next(&mut self, now: TimeStamp) -> Option<Component>;

    fn dispatch_component(&mut self, 
        c: Component, now: TimeStamp) -> EnqueueResult;

    fn is_1(&self) -> bool;

    fn produces(&self, product: Product) -> bool {
        match self.is_1() {
            true => matches!(product, Product::P1(_, _)),
            false => {
                match product {
                    Product::P1(..) => false,
                    Product::P2(..) => true, 
                    Product::P3(..) => true
                }
            }
        }
    } 

    fn has_component(&self) -> bool;

    fn take_held_component(&mut self) -> Option<Component>;

    fn is_blocked(&self) -> bool {
        match self.next_end_time() {
            Some(_) => false,
            None    => self.has_component()
        }
    }

    fn next_end_time(&self) -> Option<TimeStamp>; 

    fn name(&self) -> &str {
        match self.is_1() {
            true => "Inspector1",
            false => "Inspector2"
        }
    }
}


pub struct Inspector1 {
    ws: [Rc<RefCell<Workstation>>; 3],
    durations_c1: VecDeque<Duration>,
    held_component: Option<Component>,
    next_finish_time: Option<TimeStamp>
}

impl Inspector1 {
    pub fn new(ws: [Rc<RefCell<Workstation>>; 3], 
            durations_c1: VecDeque<Duration>) -> Self {
        Inspector1 {
            ws,
            durations_c1,
            held_component: None,
            next_finish_time: None
        }
    }
}

impl Inspector for Inspector1 {

    fn take_held_component(&mut self) -> Option<Component> {
        match self.held_component {
            Some(_) => {
                let c = self.held_component;
                self.held_component = None;
                self.next_finish_time = None;
                return c;
            },
            None => None
        }
    }

    fn has_component(&self) -> bool {
        matches!(self.held_component, Some(_))
    }

    fn is_1(&self) -> bool { true }

    fn dispatch_component(&mut self, 
            c: Component, now: TimeStamp) -> EnqueueResult {
        // moves component into the workstation 
        // with the least number of components
        // in waiting
        if self.is_blocked() {
            panic!("dispatch for Inspector1 should never be called while blocked");
        }
        assert!(matches!(c, Component::C1(..)));

        let mut ws1 = self.ws[0].borrow_mut();
        let mut ws2 = self.ws[1].borrow_mut();
        let mut ws3 = self.ws[2].borrow_mut();
        let awaiting = [
            ws1.unprocessed_components(), 
            ws2.unprocessed_components(), 
            ws3.unprocessed_components()];
    
        if awaiting[0] >= awaiting[1] &&  awaiting[0] >= awaiting[2] {
            return ws1.enqueue(true, c, now);
        } else if awaiting[1] >= awaiting[2] {
            return ws2.enqueue(true, c, now);
        } else {
            return ws3.enqueue(true, c, now);
        }
    }

    fn inspect_next(&mut self, now: TimeStamp) -> Option<Component> {
        match self.durations_c1.pop_front() {
            Some(duration) => {
                self.next_finish_time = Some(now + duration);
                let component = Some(Component::C1(duration, Some(now), None));
                self.held_component = component;
                component
            },
            None => None
        }
    }

    fn next_end_time(&self) -> Option<TimeStamp> {
        self.next_finish_time
    }
}

pub struct Inspector2 {
    ws: [Rc<RefCell<Workstation>>; 2],
    durations_c2: VecDeque<Duration>,
    durations_c3: VecDeque<Duration>,
    held_component: Option<Component>,
    next_finish_time: Option<TimeStamp>,
    random: Random
}

impl Inspector2 {
    pub fn new(
            ws: [Rc<RefCell<Workstation>>; 2],
            durations_c2: VecDeque<Duration>,
            durations_c3: VecDeque<Duration>) -> Self {
        Inspector2 {
            ws,
            durations_c2,
            durations_c3,
            held_component: None,
            next_finish_time: None,
            random: Random::new()
        }
    }

    fn decide_next_component(&mut self) -> Option<Component> {
        if self.durations_c2.len() == 0 && self.durations_c3.len() != 0 {
            return Some(
                Component::C3(self.durations_c3.pop_front().unwrap(), None, None));
        } else if self.durations_c2.len() != 0 && self.durations_c3.len() == 0 {
            return Some(
                Component::C2(self.durations_c2.pop_front().unwrap(), None, None));
        } else if self.durations_c2.len() != 0 && self.durations_c3.len() != 0 {
            return match self.random.boolean() {
                true => Some(
                    Component::C3(
                        self.durations_c3.pop_front().unwrap(), 
                        None, None)),
                false => Some(
                    Component::C2(
                        self.durations_c2.pop_front().unwrap(), 
                        None, None))
            } 
        }
        None
    }
}

impl Inspector for Inspector2 {

    fn is_1(&self) -> bool { false }

    fn take_held_component(&mut self) -> Option<Component> {
        let c = self.held_component;
        self.held_component = None;
        self.next_finish_time = None;
        return c;
    }

    fn has_component(&self) -> bool {
        matches!(self.held_component, Some(_))
    }

    fn dispatch_component(&mut self, 
            c: Component, now: TimeStamp) -> EnqueueResult {
        if self.is_blocked() {
            panic!("Inspector should never be called while blocked");
        }

        match c {
            Component::C2(..) => self.ws[0].borrow_mut().enqueue(false, c, now),
            Component::C3(..) => self.ws[1].borrow_mut().enqueue(false, c, now),
            Component::C1(..) => panic!("Inspector 2 does not inspect Component 1")
        }
    }

    fn inspect_next(&mut self, now: TimeStamp) -> Option<Component> {
        match self.decide_next_component() {
            Some(mut component) => {
                component.start_inspecting(now);
                self.next_finish_time = Some(now + component.duration());
                let component = Some(component);
                self.held_component = component;
                component
            },
            None => None
        }
    }

    fn next_end_time(&self) -> Option<TimeStamp> {
        self.next_finish_time
    }
}


impl SimulationActor for &mut dyn Inspector {
    fn respond_to(&mut self, event: FacilityEvent) -> Vec<FacilityEvent> {
        match event {

            // if a workstation assembled a component
            // then the inspector may no longer be blocked, so
            // the inspector should return an event to enqueue
            // an item if they hold a component
            FacilityEvent::Assembled(product, _ws) => {
                match self.produces(product) {
                    true => {
                        match self.take_held_component() {
                            Some(c) => {
                                match FacilityEvent::inspector_tries_unblock_self(
                                        self.dispatch_component(c, event.timestamp()),
                                        product.timestamp()) {
                                    Some(follow_up_events) => follow_up_events.to_vec(),
                                    None => Vec::default()  
                                }
                            }
                            None => Vec::default() // the inspector may not have been given anything yet
                        }
                    },
                    false => Vec::default()
                }
            },

            FacilityEvent::SimulationStarted => {
                vec!(FacilityEvent::StartedInspection(
                    self.is_1(), 
                    self.inspect_next(event.timestamp())
                        .expect(
                            format!("Failure loading inspection times for {}", self.name()).as_str())
                    ))
            },

            FacilityEvent::FinishedInspection(ins1, mut component) => {

                let r = match self.is_1() == ins1 {
                    true => FacilityEvent::inspector_places(
                        self.dispatch_component(component, 
                            component.inspection_end_time())).to_vec(),
                    false => Vec::default()
                };
                println!("RIGHT HERE {:?}", r);
                r
            },

            _ => Vec::default()
        }
    }

    fn respond(&mut self, now: TimeStamp, duration: Duration) -> Vec<FacilityEvent> {
        //assert!(self.duration_until_next_event(now).unwrap_or(0) == 0);
        println!("responding: {:?}", self.name());
        let mut component = self.take_held_component().expect("wrong inspector");
        component.finish_inspecting(now);
        vec!(FacilityEvent::FinishedInspection(
            self.is_1(), component))
    }
    
    fn duration_until_next_event(&self, now: TimeStamp) -> Option<Duration> {
        match self.is_blocked() {
            true => None,
            false => {
                match self.next_end_time() {
                   Some(ts) => Some(ts - now),
                   None => None 
                }
            }
        }
    }
}