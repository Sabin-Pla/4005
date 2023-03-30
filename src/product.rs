use crate::simulation::TimeStamp;
use crate::component::Component;

#[derive(Copy, Clone, Debug)]
pub enum Product {
    P1(Component, TimeStamp), // C1
    P2(Component, Component, TimeStamp), // C1, C2
    P3(Component, Component, TimeStamp)  // C1, C3
}

impl Product {
    fn new_p1(c1: Component, timestamp: TimeStamp) -> Self {
        assert!(matches!(c1, Component::C1(..)));
        Product::P1(c1, timestamp)
    }

    fn new_p2(c1: Component, c2: Component, timestamp: TimeStamp) -> Self {
        assert!(matches!(c1, Component::C1(..)));
        assert!(matches!(c2, Component::C2(..)));
        Product::P2(c1, c2, timestamp)
    }

    fn new_p3(c1: Component, c3: Component, timestamp: TimeStamp) -> Self {
        assert!(matches!(c1, Component::C1(..)));
        assert!(matches!(c3, Component::C3(..)));
        Product::P3(c1, c3, timestamp)
    }

    pub fn from(
            first: Component, 
            second: Option<Component>,
            timestamp: TimeStamp) -> Self {

        match second {
            None => Self::new_p1(first, timestamp),
            Some(c) => {
                match c {
                    Component::C2(..) => Self::new_p2(first, c, timestamp),
                    Component::C3(..) => Self::new_p3(first, c, timestamp),
                    _ => panic!()
                }
            }
        }
    }

    pub fn timestamp(&self) -> TimeStamp {
        match self {
            Product::P1(_, ts) => *ts,
            Product::P2(_, _, ts) => *ts,
            Product::P3(_, _, ts) => *ts
        }
    }
}
