use std::time::Duration;

use korangar::ClientHooks;
use korangar::input::InputEvent;
use korangar::state::ClientState;
use korangar_networking::NetworkEvent;
use rust_state::{Path, State};

pub struct Timer {}

impl Timer {
    fn reset(&mut self) {}

    fn elapsed_time(&self) -> Duration {
        Duration::from_millis(10)
    }
}

pub enum StepFlow {
    Continue,
    Success,
    Failure,
}

pub trait WorkStep {
    fn inject_input_event(&self, input_events: &mut Vec<InputEvent>) -> StepFlow {
        let _ = input_events;
        StepFlow::Continue
    }

    fn inspect_network_event(&self, network_event: &NetworkEvent, timer: &Timer) -> StepFlow {
        let _ = network_event;
        let _ = timer;
        StepFlow::Continue
    }

    fn inspect_state(&self, state: &mut State<ClientState>, timer: &Timer) -> StepFlow {
        let _ = state;
        let _ = timer;
        StepFlow::Continue
    }
}

pub fn modify_state<T>(path: impl Path<ClientState, T>, value: T) -> Box<dyn WorkStep>
where
    T: Clone + 'static,
{
    struct Inner<T, P> {
        path: P,
        value: T,
    }

    impl<T, P> WorkStep for Inner<T, P>
    where
        T: Clone,
        P: Path<ClientState, T>,
    {
        fn inspect_state(&self, state: &mut State<ClientState>, timer: &Timer) -> StepFlow {
            *state.follow_mut(self.path) = self.value.clone();
            let _ = timer;
            StepFlow::Success
        }
    }

    Box::new(Inner { path, value })
}

pub fn inject_input(input_event: InputEvent) -> Box<dyn WorkStep> {
    struct Inner(InputEvent);

    impl WorkStep for Inner {
        fn inject_input_event(&self, input_events: &mut Vec<InputEvent>) -> StepFlow {
            input_events.push(self.0.clone());
            StepFlow::Success
        }
    }

    Box::new(Inner(input_event))
}

pub fn wait_for_network_event_with(f: impl Fn(&NetworkEvent) -> bool + 'static) -> Box<dyn WorkStep> {
    struct Inner<F> {
        f: F,
    }

    impl<F> WorkStep for Inner<F>
    where
        F: Fn(&NetworkEvent) -> bool,
    {
        fn inspect_network_event(&self, network_event: &NetworkEvent, timer: &Timer) -> StepFlow {
            if (self.f)(network_event) {
                StepFlow::Success
            } else if timer.elapsed_time() > Duration::from_millis(500) {
                StepFlow::Failure
            } else {
                StepFlow::Continue
            }
        }
    }

    Box::new(Inner { f })
}

pub fn wait_for_network_event_or_failure_with(
    f: impl Fn(&NetworkEvent) -> bool + 'static,
    e: impl Fn(&NetworkEvent) -> bool + 'static,
) -> Box<dyn WorkStep> {
    struct Inner<F, E> {
        f: F,
        e: E,
    }

    impl<F, E> WorkStep for Inner<F, E>
    where
        F: Fn(&NetworkEvent) -> bool,
        E: Fn(&NetworkEvent) -> bool,
    {
        fn inspect_network_event(&self, network_event: &NetworkEvent, timer: &Timer) -> StepFlow {
            println!("Comparing network events: {:?}", network_event);

            if (self.f)(network_event) {
                StepFlow::Success
            } else if (self.e)(network_event) || timer.elapsed_time() > Duration::from_millis(500) {
                StepFlow::Failure
            } else {
                StepFlow::Continue
            }
        }
    }

    Box::new(Inner { f, e })
}

pub struct TestManager {
    steps: Vec<Box<dyn WorkStep>>,
    current_step: usize,
    step_timer: Timer,
    error_message: Option<String>,
}

impl TestManager {
    pub fn new(steps: Vec<Box<dyn WorkStep>>) -> Self {
        Self {
            steps,
            current_step: 0,
            step_timer: Timer {},
            error_message: None,
        }
    }

    fn do_for_current_step(&mut self, f: impl FnOnce(&dyn WorkStep, &Timer) -> StepFlow) {
        let Some(current_step) = self.steps.get(self.current_step) else {
            return;
        };

        match f(&**current_step, &self.step_timer) {
            StepFlow::Continue => {}
            StepFlow::Success => {
                self.current_step += 1;
                self.step_timer.reset();
                println!("Advancing to the next step: {}", self.current_step);
            }
            StepFlow::Failure => {
                self.error_message = Some("tests failed".to_owned());
                println!(">>>>>>>> Test failed");
                // TODO: Should terminate another way.
                self.current_step += 1;
            }
        }
    }
}

impl ClientHooks for TestManager {
    fn inject_input_event(&mut self, input_events: &mut Vec<InputEvent>) {
        self.do_for_current_step(|step, _| step.inject_input_event(input_events));
    }

    fn inspect_network_event(&mut self, network_event: &NetworkEvent) {
        self.do_for_current_step(|step, timer| step.inspect_network_event(network_event, timer));
    }

    fn inspect_state(&mut self, state: &mut State<ClientState>) {
        self.do_for_current_step(|step, timer| step.inspect_state(state, timer));
    }
}
