use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter, Result};
use std::rc::Rc;

#[macro_use]
mod logging {
    macro_rules! log {
        // a more convenient entry point for
        // the println! macro so that we can easily
        // disable printing of the simulation activity
        // whenever needed

        ($($tts:tt)*) => {
            #[cfg(debug_assertions)] {
                println!($($tts)*);
            }
        };
    }
}

mod component;
mod event;
mod inspector;
mod product;
mod random;
mod simulation;
mod workstation;

use component::Component;
use event::FacilityEvent;
use event::FacilityEvent::*;
use inspector::*;
use product::Product;
use random::Random;
use simulation::Duration;
use simulation::SimulationActor;
use simulation::TimeStamp;
use workstation::Type as WSType;
use workstation::Workstation;

// this enum is basically just a spaghetti code handler
// to work around the fact that I couldn't figure out how to coerce
// the Workstation and Inspector structs into SimulationActors
// and just have actors be a Rc<RefCell<dyn SimulationActor>>
enum Actor<'a> {
    Ptr(&'a mut dyn Inspector),
    Val(Rc<RefCell<Workstation>>),
}

impl<'a> Actor<'a> {
    fn ptr(p: &'a mut dyn Inspector) -> Self {
        Actor::Ptr(p)
    }

    fn val(v: Rc<RefCell<Workstation>>) -> Self {
        Actor::Val(v)
    }

    fn respond_to(&mut self, event: FacilityEvent) -> Option<FacilityEvent> {
        match self {
            Actor::Ptr(p) => (p as &mut dyn SimulationActor).respond_to(event),
            Actor::Val(v) => v.borrow_mut().respond_to(event),
        }
    }

    fn respond(&mut self, now: TimeStamp) -> Option<FacilityEvent> {
        match self {
            Actor::Ptr(p) => (p as &mut dyn SimulationActor).respond(now),
            Actor::Val(v) => v.borrow_mut().respond(now),
        }
    }

    fn duration_until_next_event(&self, now: TimeStamp) -> Option<Duration> {
        match self {
            Actor::Ptr(p) => (p as &dyn SimulationActor).duration_until_next_event(now),
            Actor::Val(v) => v.borrow_mut().duration_until_next_event(now),
        }
    }
}

impl<'a> Display for Actor<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Ptr(ins) => write!(f, "{}", ins),
            Self::Val(ws) => write!(f, "{}", ws.borrow()),
        }
    }
}

struct FacilitySimulation<'a> {
    actors: Vec<Actor<'a>>,
    clock: TimeStamp,
}

impl FacilitySimulation<'_> {
    fn time_until_next_actor_event(&self, now: TimeStamp) -> Option<(usize, Duration)> {
        // returns the index of the actor
        // who will produce the next event
        // along with the duration until that event
        // should be dispatched

        let response_times: Vec<Duration> = self
            .actors
            .iter()
            .map(|a| {
                a.duration_until_next_event(now)
                    .unwrap_or(Duration::never())
            })
            .collect();
        let mut min = (0, Duration::never(), false);
        for (idx, val) in response_times.iter().enumerate() {
            log!("actor: {:?}, {}\t{}", idx, val, self.actors[idx]);
            if !val.as_minutes().is_infinite() && val < &min.1 {
                min.0 = idx;
                min.1 = *val;
                min.2 = true;
            }
        }
        log!("Next event in: {}, actor: {}\n\n", min.1, min.0);
        match min.2 {
            false => None,
            true => Some((min.0, min.1)),
        }
    }

    pub fn run(mut self) -> Duration {
        // runs a simulation to completion, consuming this
        // simulation structure and returns a vector of
        // the events which are tracked as output events
        self.dispatch_to_simulation_actors(SimulationStarted);
        while let Some((next_actor_index, duration)) = self.time_until_next_actor_event(self.clock)
        {
            self.clock += duration;
            log!("Time: {}", self.clock);
            let responses = self.actors[next_actor_index].respond(self.clock);
            for response in responses.into_iter() {
                self.dispatch_to_simulation_actors(response);
            }
        }
        self.clock - TimeStamp::start()
    }

    fn dispatch_to_simulation_actors(&mut self, event: FacilityEvent) {
        for actor in self.actors.iter_mut() {
            actor.respond_to(event);
        }
    }
}

fn get_assembly_durations(rand: &mut Random) -> [VecDeque<Duration>; 3] {
    const WS1_LAMBDA: f64 = 0.217;
    const WS2_LAMBDA: f64 = 0.090;
    const WS3_LAMBDA: f64 = 0.114;

    let inverse_cdf = |x: f64, lambda: f64| Duration::of_minutes(-(1.0 - x).ln() / lambda);

    let ws1_dur: Vec<Duration> = (0..COMPONENT_COUNT)
        .map(|_| inverse_cdf(rand.float(), WS1_LAMBDA))
        .collect();
    let ws2_dur: Vec<Duration> = (0..COMPONENT_COUNT)
        .map(|_| inverse_cdf(rand.float(), WS2_LAMBDA))
        .collect();
    let ws3_dur: Vec<Duration> = (0..COMPONENT_COUNT)
        .map(|_| inverse_cdf(rand.float(), WS3_LAMBDA))
        .collect();
    [ws1_dur.into(), ws2_dur.into(), ws3_dur.into()]
}

fn get_inspection_durations(mut rand: Random) -> [VecDeque<Duration>; 3] {
    const I1_1_LAMBDA: f64 = 0.097;
    const I2_2_LAMBDA: f64 = 0.064;
    const I2_3_LAMBDA: f64 = 0.048;

    let inverse_cdf = |x: f64, lambda: f64| Duration::of_minutes(-(1.0 - x).ln() / lambda);

    let ins1_1: Vec<Duration> = (0..COMPONENT_COUNT)
        .map(|_| inverse_cdf(rand.float(), I1_1_LAMBDA))
        .collect();
    let ins2_2: Vec<Duration> = (0..COMPONENT_COUNT)
        .map(|_| inverse_cdf(rand.float(), I2_2_LAMBDA))
        .collect();
    let ins2_3: Vec<Duration> = (0..COMPONENT_COUNT)
        .map(|_| inverse_cdf(rand.float(), I2_3_LAMBDA))
        .collect();
    [ins1_1.into(), ins2_2.into(), ins2_3.into()]
}

fn buffer_stats(
    ws: Rc<RefCell<Workstation>>,
    component: Component,
    start_time: f64,
    end_time: f64,
) -> f64 {
    // prints various buffer stas and returns the average occupancy
    let ws = ws.borrow();

    let count_in_ws = |component: Component, w: WSType| match w
        .first_enqueue_time()
        .unwrap_or(TimeStamp::start())
        .get()
        > start_time
    {
        true => w.matching_count(component) as f64,
        false => 0.0,
    };

    let count_arrival = |component: Component, w: &[(TimeStamp, WSType)]| {
        // match if the number of components between state 2 and 1
        // increased (i.e, if there was an arrival)
        match count_in_ws(component, w[0].1) < count_in_ws(component, w[1].1) {
            true => 1.0,
            false => 0.0,
        }
    };

    let products = ws
        .products
        .iter()
        .filter(|p| p.start_time().get() > start_time)
        .collect::<Vec<&Product>>();

    let occupancy = (1.0 / (end_time - start_time))
        * ws.buffer_states.as_slice().windows(2).fold(0.0, |acc, w| {
            acc + (w[1].0 - w[0].0).as_minutes() * count_in_ws(component, w[0].1)
        });

    log!(
        "L = average {component} occupancy of {} {:.2}",
        ws.name(),
        occupancy
    );
    let arrival_rate = (1.0 / (end_time - start_time))
        * ws.buffer_states
            .as_slice()
            .windows(2)
            .fold(0.0, |acc, w| acc + count_arrival(component, w));
    log!(
        "λ = {component} buffer entry throughput  of {} {:.5}",
        ws.name(),
        arrival_rate
    );
    let wait_time = (1.0 / products.len() as f64)
        * products.iter().fold(Duration::none(), |acc, product| {
            acc + product.wait_time(component)
        });
    log!(
        "W = average {component} wait time of {} {:.2}",
        ws.name(),
        wait_time
    );
    log!(
        "L - λW = {:.9}",
        occupancy - arrival_rate * wait_time.as_minutes()
    );
    occupancy
}

fn ws_stats(ws: Rc<RefCell<Workstation>>, total_time: f64) -> f64 {
    let ws = ws.borrow();

    let add_working_time = |bs: &[(TimeStamp, WSType)]| {
        // println!("{} {} {}", bs[1].0, bs[0].0, bs[0].1 );
        match bs[0].1.can_work() {
            true => bs[1].0 - bs[0].0,
            false => Duration::none(),
        }
    };

    ws.buffer_states
        .windows(2)
        .fold(Duration::none(), |acc, bs| acc + add_working_time(bs))
        .as_minutes()
        / total_time
}

fn product_stats(p: Vec<Product>, start_time: f64, end_time: f64) -> f64 {
    // calculates product throughput
    log!("Total {}: {}", p[0].name(), p.len());
    p.len() as f64 / (end_time - start_time)
}

fn inspector_stats(ins: &dyn Inspector, start_time: f64, end_time: f64) -> f64 {
    // gets the proportion of time for which an inspector was blocked.
    let mut i = 0;
    let mut add_next_slice = |w: &[&TimeStamp]| {
        i += 1;
        match i % 2 == 0 {
            false => *w[1] - *w[0],
            true => Duration::none(),
        }
    };

    ins.blocked_times()
        .iter()
        .filter(|ts| ts.get() < end_time && ts.get() > start_time)
        .collect::<Vec<&TimeStamp>>()
        .as_slice()
        .windows(2)
        .fold(Duration::none(), |acc, w| acc + add_next_slice(w))
        .as_minutes()
        / (end_time - start_time)
}

fn littles_law_whole_system(
    inspection_times: [&mut VecDeque<TimeStamp>; 2],
    exit_times: [&mut VecDeque<TimeStamp>; 2],
    products: [&Vec<Product>; 3],
    start_time: f64,
    end_time: f64,
) -> f64 {
    // checks that little law holds for the whole system and
    // returns the total average buffer occupancy

    // combine all the products into one list
    // and filter those occuring before start_time
    let products = products
        .into_iter()
        .flatten()
        .collect::<Vec<&Product>>()
        .into_iter()
        .filter(|p| (p.timestamp().get() < end_time) && (p.start_time().get() > start_time))
        .collect::<Vec<&Product>>();

    // get a count of all components required to make all products
    // which were assembled after start_time
    let total_component_count = products.iter().map(|p| p.component_count()).sum::<usize>() as f64;

    // get the summation of component in system time and divide it by
    // the number of components to get the average wait time per component
    let wait_time: f64 = (1.0 / total_component_count)
        * products
            .iter()
            .fold(Duration::none(), |acc, p| {
                acc + p.time_components_in_system()
            })
            .as_minutes();

    let min_idx = |v: &[&mut VecDeque<TimeStamp>; 4]| -> Option<usize> {
        let idx = v
            .iter()
            .enumerate()
            .min_by(|(_, v0), (_, v1)| v0.partial_cmp(&v1).unwrap())
            .map(|(idx, _)| idx)
            .unwrap();
        match v[idx].front() {
            Some(_) => Some(idx),
            None => None,
        }
    };

    let v = [
        &mut inspection_times[0].clone(),
        &mut inspection_times[1].clone(),
        &mut exit_times[0].clone(),
        &mut exit_times[1].clone(),
    ];
    let mut t = start_time;
    let mut current_occupants = 0;
    let mut occupancy = 0.0;
    let mut arrival_rate = (0.0, t, 0);

    // build step function calculate occupants
    while t < end_time {
        match min_idx(&v) {
            Some(idx) => {
                let t2 = v[idx].pop_front().unwrap().get();
                if idx == 0 || idx == 1 {
                    current_occupants += 1;
                    if t2 > start_time {
                        arrival_rate.0 += t2 - arrival_rate.1;
                        arrival_rate.1 = t2;
                        arrival_rate.2 += 1;
                    }
                } else if idx == 2 {
                    // P1 uses 1 C1
                    current_occupants -= 1;
                } else if idx == 3 {
                    // P2 and P3 always use 2 components
                    current_occupants -= 2;
                }
                if t > start_time {
                    occupancy += current_occupants as f64 * (t2 - t);
                }
                t += t2 - t;
            }
            None => break,
        }
    }

    occupancy = occupancy / (end_time - start_time);
    // arrival rate = total arrivals / total time between arrivals
    let arrival_rate = arrival_rate.2 as f64 / arrival_rate.0;

    log!("\nLittles law (Whole System)");
    log!("average occupancy (L) : {:.2}", occupancy);
    log!("arrival rate      (λ) : {:.2}", arrival_rate);
    log!("waiting_time      (W) : {:.2}", wait_time);
    log!(
        "              L-λW    : {:.2}",
        occupancy - arrival_rate * wait_time
    );
    occupancy
}

const INIT_R: usize = 10;
const MAX_R: usize = 200;
pub const COMPONENT_COUNT: usize = 3000;

fn run_iteration(seed: u32, start_time: f64) -> [Vec<f64>; 5] {
    let mut rand = Random::with_seed(seed);
    let assembly_durations: [VecDeque<Duration>; 3] = get_assembly_durations(&mut rand);
    let ws1 = Rc::new(RefCell::new(Workstation::new(
        WSType::W1([None, None]),
        assembly_durations[0].clone(),
    )));
    let ws2 = Rc::new(RefCell::new(Workstation::new(
        WSType::W2([None, None], [None, None]),
        assembly_durations[1].clone(),
    )));
    let ws3 = Rc::new(RefCell::new(Workstation::new(
        WSType::W3([None, None], [None, None]),
        assembly_durations[2].clone(),
    )));

    let inspect_durations: [VecDeque<Duration>; 3] = get_inspection_durations(rand);
    let mut inspector1 = Inspector1::new(
        [ws1.clone(), ws2.clone(), ws3.clone()],
        inspect_durations[0].clone(),
    );
    let mut inspector2 = Inspector2::new(
        [ws2.clone(), ws3.clone()],
        inspect_durations[1].clone(),
        inspect_durations[2].clone(),
    );

    let actors: Vec<Actor> = vec![
        Actor::val(ws1.clone()),
        Actor::val(ws2.clone()),
        Actor::val(ws3.clone()),
        Actor::ptr(&mut inspector1 as &mut dyn Inspector),
        Actor::ptr(&mut inspector2 as &mut dyn Inspector),
    ];
    let simulation = FacilitySimulation {
        actors,
        clock: TimeStamp::start(),
    };

    // run the simulation and get the first a workstation
    // finished its last product as the end time
    simulation.run();
    let last_ws_log =
        |ws: &Rc<RefCell<Workstation>>| ws.borrow().products.last().unwrap().timestamp().get();
    let end_time = *[last_ws_log(&ws3), last_ws_log(&ws1), last_ws_log(&ws2)]
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    log!(
        "Finished simulation start: {:.2} end: {:.2}", start_time, end_time
    );

    // calculate stats for all 5 buffers
    let buffer_stats = [
        buffer_stats(
            ws1.clone(),
            Component::new(Duration::never(), 1),
            start_time,
            end_time,
        ),
        buffer_stats(
            ws2.clone(),
            Component::new(Duration::never(), 1),
            start_time,
            end_time,
        ),
        buffer_stats(
            ws2.clone(),
            Component::new(Duration::never(), 2),
            start_time,
            end_time,
        ),
        buffer_stats(
            ws3.clone(),
            Component::new(Duration::never(), 1),
            start_time,
            end_time,
        ),
        buffer_stats(
            ws3.clone(),
            Component::new(Duration::never(), 3),
            start_time,
            end_time,
        ),
    ];
    log!("Average buffer occupancies: {:.2?}", buffer_stats);

    // calculate stats for each WS
    let ws_stats = [
        ws_stats(ws1.clone(), end_time),
        ws_stats(ws2.clone(), end_time),
        ws_stats(ws3.clone(), end_time),
    ];
    log!("WS working rate [WS1, WS2, WS3] {:.2?}", ws_stats);

    // calculate stats for each product
    let product_stats = [
        product_stats(ws1.borrow().products.clone(), start_time, end_time),
        product_stats(ws2.borrow().products.clone(), start_time, end_time),
        product_stats(ws3.borrow().products.clone(), start_time, end_time),
    ];
    log!("Product throughput [P1, P2, P3] {:.2?}", product_stats);

    // calculate stats for each inspector
    let inspector_stats = [
        inspector_stats(&inspector1, start_time, end_time),
        inspector_stats(&inspector2, start_time, end_time),
    ];
    log!(
        "Inspector blocked rate [Ins1, Ins2] {:.2?}",
        inspector_stats
    );

    let ins1 = inspector1.inspection_times();
    let ins2 = inspector2.inspection_times();
    let total_average_occupancy = littles_law_whole_system(
        [ins1.0, ins2.0],
        [ins1.1, ins2.1],
        [
            &ws1.borrow().products,
            &ws2.borrow().products,
            &ws3.borrow().products,
        ],
        start_time,
        end_time,
    );

    [
        buffer_stats.to_vec(),
        ws_stats.to_vec(),
        product_stats.to_vec(),
        inspector_stats.to_vec(),
        vec!(total_average_occupancy)
    ]
}

fn std_dev(v: &Vec<f64>) -> f64 {
    match v.len() < INIT_R {
        true => f64::INFINITY,
        false => {
            let avg: f64 = v.iter().sum::<f64>() / v.len() as f64;
            v.iter().fold(0.0, |acc, vi| acc + (vi - avg).abs()) / (v.len() as f64 - 1.0)
        }
    }
}

fn main() {

    let mut i = 0;
    let start_time = 600.0;
    let mut cumulative_stats = [vec![0.0; 5], vec![0.0; 3], 
        vec![0.0; 3], vec![0.0; 2], vec![0.0]].to_vec();

    let e = 0.02; // 𝜀
    let z_025 = 1.960; // 95% confidence
    let mut y: Vec<Vec<f64>> = vec![vec!(); 14];
    let mut cumulative_std_dev: Vec<Vec<f64>> = vec![vec!(); 14];
    let mut n = 0;

    let calculate_total_throughput = |throughput: Vec<f64>| {
        (throughput[0] + throughput[1] * 2.0 + throughput[2] * 2.0) / 3.0
    };

    for r in 0..MAX_R {
        n += 1;
        let stats = run_iteration(n as u32, start_time);
        let total_throughput = 
            calculate_total_throughput(stats[2].clone());
        println!("{n} \t {total_throughput}");
        for (i, v) in stats.iter().enumerate() {
            for i2 in 0..v.len() {
                cumulative_stats[i][i2] += v[i2];
            }
        }

        // number to compare against n to determine
        // the number of replications
        let mut calculated_r: Vec<f64> = vec![];

        for (i, stat) in stats
            .into_iter()
            .flatten()
            .collect::<Vec<f64>>()
            .into_iter()
            .enumerate()
        {
            y[i].push(stat);
            calculated_r.push((std_dev(&y[i]) * z_025 / e).powf(2.0));
            cumulative_std_dev[i].push(std_dev(&y[i]));
        }

        if r > INIT_R && !(calculated_r.iter().any(|cr| *cr as f64 > r as f64)) {
            println!("\nConverged on replication count (R) of {r}");
            break;
        }
    }


    let mut cumulative_stats = cumulative_stats.into_iter().flatten().collect::<Vec<f64>>();
    for i in 0..cumulative_stats.len() {
        cumulative_stats[i] = cumulative_stats[i] / n as f64;
    }

    let t_025 = [
        2.262, 2.228, 2.201, 2.179, 
        2.160, 2.145, 2.131, 2.120, 2.110,
        2.101, 2.093, 2.086, 2.080, 2.074, 2.069,
        2.064, 2.060, 2.056, 2.052, 2.048, 2.045,
        2.042];


    let bound = |i: usize| -> f64 {
        match n < 31 {
            true => t_025[n-10] * cumulative_std_dev[i][cumulative_std_dev[i].len() - 1] / (n as f64).sqrt(),
            false => z_025 * cumulative_std_dev[i][cumulative_std_dev[i].len() - 1] / (n as f64).sqrt()
        }
    };

    const BUFFER_HEADERS: [&str; 5] = [
        "C1 of WS1",
        "C1 of WS2",
        "C2 of WS2",
        "C1 of WS3",
        "C3 of WS3",
    ];
    log!(
        "\nAverages for {n} replications with a \
        queue size of {COMPONENT_COUNT} each"
    );
    log!("\nAverage occupancy for each buffer:");
    for (i, h) in BUFFER_HEADERS.iter().enumerate() {
        println!("{} {:#.2?} +- {:.5}", h, cumulative_stats[i], bound(i));
    }
    log!(
        "\n[W1, W2, W3] busy ratio : {:.2?} | CI [{:.4}, {:.4}, {:.4}]",
        &cumulative_stats[5..8],
        bound(5), bound(6), bound(7)
    );
    log!(
        "\n[P1, P2, P3] throughput : {:.2?} | CI [{:.4}, {:.4}, {:.4}] ",
        &cumulative_stats[8..11],
        bound(8), bound(9), bound(10)
    );
    log!(
        "\n[Inspector1, Inspector2] blocked ratio: {:.4?} | CI [{:.4}, {:.4}]",
        &cumulative_stats[11..13], bound(11), bound(12)
    );

    log!(
        "\n Total Average Occupancy: {:.4?}",
        &cumulative_stats[13]
    );


}
