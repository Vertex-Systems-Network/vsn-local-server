#[cfg(windows)]
mod windows_fixture {
    use std::{
        ffi::OsString,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread,
        time::Duration,
    };
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };

    pub const SERVICE_NAME: &str = "VSN-PKG02-0213";

    define_windows_service!(ffi_service_main, service_main);

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        Ok(())
    }

    fn service_main(_arguments: Vec<OsString>) {
        if let Err(error) = run_service() {
            eprintln!("pkg02_service_fixture_error={error}");
        }
    }

    fn run_service() -> Result<(), Box<dyn std::error::Error>> {
        let stop = Arc::new(AtomicBool::new(false));
        let handler_stop = Arc::clone(&stop);
        let event_handler = move |control| -> ServiceControlHandlerResult {
            match control {
                ServiceControl::Stop => {
                    handler_stop.store(true, Ordering::SeqCst);
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };
        let status = service_control_handler::register(SERVICE_NAME, event_handler)?;
        status.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;
        while !stop.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(50));
        }
        status.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;
        Ok(())
    }
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    windows_fixture::run()
}

#[cfg(not(windows))]
fn main() {
    eprintln!("pkg02_service_fixture requires Windows");
    std::process::exit(2);
}
