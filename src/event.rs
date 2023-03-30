
use crate::Component;
//use crate::Duration;
use crate::TimeStamp as TS;
use crate::workstation::Type as WS;
use crate::Product;

pub enum EnqueueResult {
    CouldEnqueue(bool, Component, WS, TS),
    Fail
}

#[derive(Copy, Clone, Debug)]
pub enum FacilityEvent {

    // an inspector has been blocked and is holding a component
    Blocked(bool, Component),

    // an inspector was unblocked and placed a component
    Unblocked(bool, TS),

    // a component was placed at a workstation
    Placement(bool, Component, WS, TS),

    // a product was assembled at a workstation
    Assembled(Product, WS),

    // a workstation has started working on a component
    WorkstationStarted(WS, TS),

    // is ins1, component, start time
    StartedInspection(bool, Component),

    // is ins1, component, end time
    FinishedInspection(bool, Component),

    SimulationStarted
}

pub enum OutputEvent {
    None,
    Test
}


impl FacilityEvent {
    pub fn inspector_tries_unblock_self(
            er: EnqueueResult,
            current_time: TS) -> Option<[FacilityEvent; 3]> {

        // when an inspector is unblocked that means they've tried to 
        // enqueue whatever component they were holding

        // this method will only ever be called by an Inspector with
        // a held item (a blocked Inspector)
        match er {
            EnqueueResult::Fail => {
                // can occur iff ins1 and ins2 both try and access ws3
                // after both being unblocked
                None //  the other inspector was the one who could unblock themselves
            },
            EnqueueResult::CouldEnqueue(ins1, c, ws, ts) => {
                let response_events = Self::inspector_places(er);
                Some([Self::Unblocked(ins1, ts), 
                    response_events[0], response_events[1]])
            }
        }
    }

    pub fn inspector_places(er: EnqueueResult) -> [FacilityEvent; 2] {
        match er {
            EnqueueResult::Fail => panic!("invalid place"),
            EnqueueResult::CouldEnqueue(ins1, c, ws, ts) =>  {
                [FacilityEvent::Placement(ins1, c, ws, ts), 
                FacilityEvent::WorkstationStarted(ws, ts)]
            }
        }
    }

    pub fn timestamp(&self) -> TS {
        match self {
            FacilityEvent::Blocked(_, mut component) => 
                component.inspection_end_time(),
            FacilityEvent::Unblocked(_, ts) => *ts,
            FacilityEvent::Placement(_, _, _, ts) => *ts,
            FacilityEvent::Assembled(product, _) => product.timestamp(),
            FacilityEvent::WorkstationStarted(_, ts) => *ts,
            FacilityEvent::StartedInspection(_, mut component) => 
                component.inspection_start_time(),
            FacilityEvent::FinishedInspection(_, mut component) => 
                component.inspection_end_time(),
            FacilityEvent::SimulationStarted => TS::start()
        }
    }
}