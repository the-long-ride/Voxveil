use super::*;
use std::time::{Duration, Instant};

#[test]
fn running_state_is_required_for_ready() {
    assert!(!RelayRuntimeState::Starting.is_running());
    assert!(RelayRuntimeState::Running.is_running());
    assert!(!RelayRuntimeState::Faulted("device removed".into()).is_running());
}

#[test]
fn route_identity_collision_is_rejected_before_spawn() {
    let spec = RelaySpec { source_endpoint_id: "same".into(), physical_output_endpoint_id: "same".into() };
    assert!(RelayHandle::validate_spec(&spec).is_err());
}

#[test]
fn distinct_route_is_accepted_before_spawn() {
    let spec = RelaySpec { source_endpoint_id: "cable".into(), physical_output_endpoint_id: "speakers".into() };
    assert_eq!(RelayHandle::validate_spec(&spec), Ok(()));
}

#[test]
fn fake_worker_can_reach_running_and_stop_cleanly() {
    let spec = RelaySpec { source_endpoint_id: "cable".into(), physical_output_endpoint_id: "speakers".into() };
    let mut handle = RelayHandle::spawn_with_worker(spec, 50, |_spec, _level, commands, state| {
        *state.lock().unwrap() = RelayRuntimeState::Running;
        if matches!(commands.recv().unwrap(), RelayCommand::Stop) { *state.lock().unwrap() = RelayRuntimeState::Stopped; }
    }).unwrap();
    handle.stop().unwrap();
    assert_eq!(handle.state(), RelayRuntimeState::Stopped);
}

#[test]
fn profile_is_hot_switched_through_the_running_worker() {
    let spec = RelaySpec { source_endpoint_id: "cable".into(), physical_output_endpoint_id: "speakers".into() };
    let observed = Arc::new(Mutex::new(Vec::new()));
    let worker_observed = Arc::clone(&observed);
    let mut handle = RelayHandle::spawn_with_worker_profile(
        spec, 50, ClassicSuppressionProfile::MusicPreservation,
        move |_spec, _level, initial_profile, commands, state| {
            worker_observed.lock().unwrap().push(initial_profile);
            *state.lock().unwrap() = RelayRuntimeState::Running;
            loop {
                match commands.recv().unwrap() {
                    RelayCommand::SetSuppressionProfile(profile) => worker_observed.lock().unwrap().push(profile),
                    RelayCommand::Stop => break,
                    RelayCommand::SetVocalLevel(_) => {}
                }
            }
        },
    ).unwrap();
    handle.set_suppression_profile(ClassicSuppressionProfile::Balanced).unwrap();
    handle.stop().unwrap();
    assert_eq!(*observed.lock().unwrap(), vec![ClassicSuppressionProfile::MusicPreservation, ClassicSuppressionProfile::Balanced]);
}

#[test]
fn normal_worker_return_cannot_leave_backend_running() {
    let spec = RelaySpec { source_endpoint_id: "cable".into(), physical_output_endpoint_id: "speakers".into() };
    let mut handle = RelayHandle::spawn_with_worker(spec, 50, |_spec, _level, _commands, state| {
        *state.lock().unwrap() = RelayRuntimeState::Running;
    }).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if handle.state() == RelayRuntimeState::Stopped { break; }
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(handle.state(), RelayRuntimeState::Stopped);
    handle.stop().unwrap();
}

#[test]
fn worker_panic_cannot_leave_backend_running() {
    let spec = RelaySpec { source_endpoint_id: "cable".into(), physical_output_endpoint_id: "speakers".into() };
    let mut handle = RelayHandle::spawn_with_worker(spec, 50, |_spec, _level, _commands, state| {
        *state.lock().unwrap() = RelayRuntimeState::Running;
        panic!("simulated relay panic");
    }).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if matches!(handle.state(), RelayRuntimeState::Faulted(_)) { break; }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(handle.state(), RelayRuntimeState::Faulted(_)));
    handle.stop().unwrap();
}
