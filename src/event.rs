
use crate::Component;
//use crate::Duration;
use crate::TimeStamp as TS;
use crate::workstation::Type as WS;
use crate::Product;

pub enum EnqueueResult {
    CouldEnqueue(bool, Component, WS, TS),
    Fail
}

#[derive(Clone, Debug)]
pub enum FacilityEvent {

    // a product was assembled at a workstation
    Assembled(Product, WS),

    // a workstation has started working on a component
    WorkstationStarted(WS, TS),

    SimulationStarted
}

pub enum OutputEvent {
    None,
    Test
}


impl FacilityEvent {
    pub fn inspector_tries_unblock_self(
            er: EnqueueResult,
            current_time: TS) -> Option<FacilityEvent> {

        // when an inspector is unblocked that means they've tried to 
        // enqueue whatever component they were holding

        // this method will only ever be called by an Inspector with
        // a held item (a blocked Inspector)
        match er {
            EnqueueResult::Fail => {
                // can occur iff ins1 and ins2 both try and access ws3
                None // in response to a WS finishing
            },
            EnqueueResult::CouldEnqueue(ins1, component, ws, ts) => { 
                assert!(ws.contains(component));
                Some(FacilityEvent::WorkstationStarted(ws, ts))
            }
        }
    }

    pub fn timestamp(&self) -> TS {
        match self {
            FacilityEvent::Assembled(product, _) => product.timestamp(),
            FacilityEvent::WorkstationStarted(_, ts) => *ts,
            FacilityEvent::SimulationStarted => TS::start()
        }
    }
}