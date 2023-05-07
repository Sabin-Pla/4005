use crate::Component;
//use crate::Duration;
use crate::workstation::Type as WS;
use crate::Product;
use crate::TimeStamp as TS;

pub enum EnqueueResult {
    CouldEnqueue(bool, Component, WS, TS, bool),
    Fail,
}

#[derive(Copy, Clone, Debug)]
pub enum FacilityEvent {
    // a product was assembled at a workstation
    Assembled(Product, WS),

    // a workstation has started working on a component
    WorkstationStarted(WS, TS),

    SimulationStarted,
}

impl FacilityEvent {
    pub fn timestamp(&self) -> TS {
        match self {
            FacilityEvent::Assembled(product, _) => product.timestamp(),
            FacilityEvent::WorkstationStarted(_, ts) => *ts,
            FacilityEvent::SimulationStarted => TS::start(),
        }
    }
}
