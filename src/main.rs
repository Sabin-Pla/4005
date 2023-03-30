use std::cell::{RefCell};
use std::rc::Rc;
use std::collections::VecDeque;

mod product;
mod workstation;
mod component;
mod inspector;
mod simulation;
mod event;
mod random;

use event::OutputEvent;
use event::FacilityEvent;
use simulation::SimulationActor;
use simulation::Duration;
use simulation::TimeStamp;
use inspector::*;
use workstation::Workstation;
use workstation::Type as WSType;
use component::Component;
use product::Product;
use event::FacilityEvent::*;
use random::Random;


enum Actor<'a> {
    Ptr(&'a mut dyn Inspector),
    Val(Rc<RefCell<Workstation>>)
}

impl<'a> Actor<'a> {
    fn ptr(p: &'a mut dyn Inspector) -> Self {
        Actor::Ptr(p)
    }

    fn val(v: Rc<RefCell<Workstation>>) -> Self {
        Actor::Val(v)
    }

    fn respond_to(&mut self, event: FacilityEvent) -> Vec<FacilityEvent> {
        match self {
            Actor::Ptr(p) => (p as &mut dyn SimulationActor).respond_to(event),
            Actor::Val(v) => v.borrow_mut().respond_to(event)
        }
    }
    fn respond(&mut self, now: TimeStamp, time_passed: Duration) -> Vec<FacilityEvent> {
        match self {
            Actor::Ptr(p) => (p as &mut dyn SimulationActor).respond(now, time_passed),
            Actor::Val(v) => v.borrow_mut().respond(now, time_passed)
        }
    }
    fn duration_until_next_event(&self, now: TimeStamp) -> Option<Duration> {
        match self {
            Actor::Ptr(p) => (p as &dyn SimulationActor).duration_until_next_event(now),
            Actor::Val(v) => v.borrow_mut().duration_until_next_event(now)
        }
    }

}

struct FacilitySimulation<'a> {
    events: VecDeque<FacilityEvent>,
    current_time: TimeStamp,
    output_events: Vec<OutputEvent>,
    actors: Vec<Actor<'a>>,
    clock: TimeStamp
}

impl FacilitySimulation<'_> {

    fn time_until_next_actor_event(&self, 
            now: TimeStamp) -> Option<(usize, Duration)> {
        // returns the index of the actor 
        // who will produce the next event
        // along with the duration until that event
        // should be dispatched

        let response_times: Vec<Duration> = self.actors
            .iter()
            .map(|a| a.duration_until_next_event(now)
                .unwrap_or(Duration::never()))
            .collect();
        let mut min = (0, Duration::never(), false);
        for (idx, val) in response_times.iter().enumerate() {
            println!("actor: {:?}, {:?}", idx, val);
            if !val.as_minutes().is_infinite() && val < &min.1 {
                min.0 = idx;
                min.1 = *val;
                min.2 = true;
            }
        }
        println!("min {:?}", min);
        match min.2 {
            false => None,
            true => Some((min.0, min.1)) 
        }
    } 

              
    pub fn run_fel(mut self) -> Vec<OutputEvent> {
        // runs a simulation to completion, consuming this
        // simulation structure and returns a vector of 
        // the events which are tracked as output events  
        self.dispatch_to_simulation_actors(SimulationStarted);
        while let Some(
                (next_actor_index, 
                duration)) = self.time_until_next_actor_event(self.clock) {
            self.clock += duration;
            println!("Time: {:?}", self.clock);
            let responses = self.actors[next_actor_index]
                .respond(self.clock, duration);
            for response in responses {
                println!("response {:?}", response);
                self.dispatch_to_simulation_actors(response);  
            } 
        }
        println!("{:#?}", self.events);
        assert!(self.events.len() == 0,
            "simulation ran to completion but unprocessed events remain");
        self.output_events
    }

    fn sort_events(events: &mut VecDeque<FacilityEvent>) {
        events.make_contiguous().sort_unstable_by(
            |a, b| a.timestamp().partial_cmp(&b.timestamp()).unwrap())
    }


    fn dispatch_to_simulation_actors(&mut self, event: FacilityEvent) {
        for actor in self.actors.iter_mut() {
            let mut responses: VecDeque<FacilityEvent> = 
                actor.respond_to(event).into();
            //self.events.append(&mut responses);
            //FacilitySimulation::sort_events(&mut self.events);
        }
    }
}

fn get_assembly_durations(rand: &mut Random) -> [VecDeque<Duration>; 3] {

    const WS1_LAMBDA: f64 = 0.217;
    const WS2_LAMBDA: f64 = 0.090;
    const WS3_LAMBDA: f64 = 0.114;
    
    let inverse_cdf = |x: f64, lambda: f64| {
        Duration::of_minutes(-(1.0 - x).ln() / lambda)
    };

    let ws1_dur: Vec<Duration> = 
        (0..300).map(|_| inverse_cdf(rand.float(), WS1_LAMBDA)).collect();
    let ws2_dur: Vec<Duration> =
        (0..300).map(|_| inverse_cdf(rand.float(), WS2_LAMBDA)).collect();
    let ws3_dur: Vec<Duration> =
        (0..300).map(|_| inverse_cdf(rand.float(), WS3_LAMBDA)).collect();
    [ws1_dur.into(), ws2_dur.into(), ws3_dur.into()]
}

fn get_inspection_durations(mut rand: Random) -> [VecDeque<Duration>; 3] {

    const I1_1_LAMBDA: f64 = 0.097;
    const I2_2_LAMBDA: f64 = 0.064;
    const I2_3_LAMBDA: f64 = 0.048;
    
    let inverse_cdf = |x: f64, lambda: f64| {
        Duration::of_minutes(-(1.0 - x).ln() / lambda)
    };

    let ins1_1: Vec<Duration> = 
        (0..300).map(|_| inverse_cdf(rand.float(), I1_1_LAMBDA)).collect();
    let ins2_2: Vec<Duration> =
        (0..300).map(|_| inverse_cdf(rand.float(), I2_2_LAMBDA)).collect();
    let ins2_3: Vec<Duration> =
        (0..300).map(|_| inverse_cdf(rand.float(), I2_3_LAMBDA)).collect();
    [ins1_1.into(), ins2_2.into(), ins2_3.into()]
}


fn main() {

    let mut rand = Random::new();
    let assembly_durations: [VecDeque<Duration>; 3] 
        = get_assembly_durations(&mut rand);
    let ws1 = Rc::new(RefCell::new(
        Workstation::new(
            WSType::W1([None, None]),
            assembly_durations[0].clone())));
    let ws2 = Rc::new(RefCell::new(
        Workstation::new(
            WSType::W2([None, None], [None, None]),
            assembly_durations[1].clone())));
    let ws3 = Rc::new(RefCell::new(
        Workstation::new(
            WSType::W3([None, None], [None, None]),
            assembly_durations[2].clone())));
    
    let ws = [ws1.clone(), ws2.clone(), ws3.clone()];

    let inspect_durations: [VecDeque<Duration>; 3] = 
        get_inspection_durations(rand);
    let mut inspector1 = Inspector1::new(
        [ws1.clone(), ws2.clone(), ws3.clone()], inspect_durations[0].clone());
    let mut inspector2 = Inspector2::new(
        [ws2.clone(), ws3.clone()], 
        inspect_durations[1].clone(), inspect_durations[2].clone());


    let actors: Vec<Actor> = vec!(
        Actor::val(ws1), 
        Actor::val(ws2), 
        Actor::val(ws3), 
        Actor::ptr(
            &mut inspector1 as &mut dyn Inspector),
        Actor::ptr(
            &mut inspector2 as &mut dyn Inspector));
    let simulation = FacilitySimulation {
        actors,
        current_time: TimeStamp::start(),
        events: VecDeque::<FacilityEvent>::with_capacity(10000),
        output_events: Vec::<OutputEvent>::with_capacity(10000),
        clock: TimeStamp::start()
    };
    let output_events = simulation.run_fel();
    println!("Complete");
}
