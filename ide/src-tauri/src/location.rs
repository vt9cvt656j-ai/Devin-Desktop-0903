use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CurrentLocationResponse {
    pub status: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub accuracy_m: Option<f64>,
    pub observed_at_unix_ms: Option<i64>,
    pub sample_age_ms: Option<u64>,
    pub source: String,
    pub message: String,
}

impl CurrentLocationResponse {
    fn failure(status: &str, source: &str, message: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            latitude: None,
            longitude: None,
            accuracy_m: None,
            observed_at_unix_ms: None,
            sample_age_ms: None,
            source: source.into(),
            message: message.into(),
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn request_current_location() -> CurrentLocationResponse {
    CurrentLocationResponse::failure(
        "unsupported",
        "native",
        "This platform does not yet have a native location provider; the caller may use an explicit browser geolocation fallback.",
    )
}

#[cfg(target_os = "macos")]
mod macos {
    use super::CurrentLocationResponse;
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_core_location::{
        kCLLocationAccuracyHundredMeters, CLAuthorizationStatus, CLError, CLLocation,
        CLLocationManager, CLLocationManagerDelegate,
    };
    use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol};
    use std::cell::{Cell, RefCell};
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::sync::oneshot;

    const LOCATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
    const MAX_SAMPLE_AGE_SECONDS: f64 = 5.0 * 60.0;
    const MAX_FUTURE_SKEW_SECONDS: f64 = 60.0;
    static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

    struct LocationDelegateIvars {
        requested_location: Cell<bool>,
        completed: Cell<bool>,
        waiters: RefCell<Vec<(u64, oneshot::Sender<CurrentLocationResponse>)>>,
    }

    fn sample_timestamps(
        seconds_from_now: f64,
        observed_seconds_since_epoch: f64,
    ) -> Option<(i64, u64)> {
        if !seconds_from_now.is_finite()
            || !observed_seconds_since_epoch.is_finite()
            || observed_seconds_since_epoch < 0.0
            || !(-MAX_SAMPLE_AGE_SECONDS..=MAX_FUTURE_SKEW_SECONDS)
                .contains(&seconds_from_now)
        {
            return None;
        }
        let observed_at_unix_ms = (observed_seconds_since_epoch * 1000.0).round() as i64;
        let sample_age_ms = ((-seconds_from_now).max(0.0) * 1000.0).round() as u64;
        Some((observed_at_unix_ms, sample_age_ms))
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[ivars = LocationDelegateIvars]
        struct LocationDelegate;

        unsafe impl NSObjectProtocol for LocationDelegate {}

        unsafe impl CLLocationManagerDelegate for LocationDelegate {
            #[unsafe(method(locationManager:didUpdateLocations:))]
            unsafe fn location_manager_did_update_locations(
                &self,
                manager: &CLLocationManager,
                locations: &NSArray<CLLocation>,
            ) {
                let Some(location) = locations.lastObject() else {
                    self.finish(
                        manager,
                        CurrentLocationResponse::failure(
                            "unavailable",
                            "core_location",
                            "macOS returned no location sample.",
                        ),
                    );
                    return;
                };
                let coordinate = unsafe { location.coordinate() };
                let accuracy = unsafe { location.horizontalAccuracy() };
                let timestamp = unsafe { location.timestamp() };
                let Some((observed_at_unix_ms, sample_age_ms)) = sample_timestamps(
                    timestamp.timeIntervalSinceNow(),
                    timestamp.timeIntervalSince1970(),
                ) else {
                    self.finish(
                        manager,
                        CurrentLocationResponse::failure(
                            "unavailable",
                            "core_location",
                            "macOS returned a stale or invalid cached location sample instead of a current position.",
                        ),
                    );
                    return;
                };
                if !coordinate.latitude.is_finite()
                    || !coordinate.longitude.is_finite()
                    || !(-90.0..=90.0).contains(&coordinate.latitude)
                    || !(-180.0..=180.0).contains(&coordinate.longitude)
                    || !accuracy.is_finite()
                    || accuracy < 0.0
                {
                    self.finish(
                        manager,
                        CurrentLocationResponse::failure(
                            "unavailable",
                            "core_location",
                            "macOS returned an invalid location sample.",
                        ),
                    );
                    return;
                }
                self.finish(
                    manager,
                    CurrentLocationResponse {
                        status: "success".into(),
                        latitude: Some(coordinate.latitude),
                        longitude: Some(coordinate.longitude),
                        accuracy_m: Some(accuracy),
                        observed_at_unix_ms: Some(observed_at_unix_ms),
                        sample_age_ms: Some(sample_age_ms),
                        source: "core_location".into(),
                        message: "Current location was provided by macOS after the user's permission decision.".into(),
                    },
                );
            }

            #[unsafe(method(locationManager:didFailWithError:))]
            unsafe fn location_manager_did_fail_with_error(
                &self,
                manager: &CLLocationManager,
                error: &NSError,
            ) {
                let code = error.code();
                let response = if code == CLError::Denied.0 || code == CLError::PromptDeclined.0 {
                    CurrentLocationResponse::failure(
                        "permission_denied",
                        "core_location",
                        "Location permission was denied. The user can enable it in System Settings > Privacy & Security > Location Services.",
                    )
                } else if code == CLError::LocationUnknown.0 {
                    CurrentLocationResponse::failure(
                        "unavailable",
                        "core_location",
                        "macOS could not determine the current location.",
                    )
                } else if code == CLError::Network.0 {
                    CurrentLocationResponse::failure(
                        "unavailable",
                        "core_location",
                        "macOS could not determine the current location because its location network was unavailable.",
                    )
                } else {
                    CurrentLocationResponse::failure(
                        "error",
                        "core_location",
                        format!("macOS Core Location failed with error code {code}."),
                    )
                };
                self.finish(manager, response);
            }

            #[unsafe(method(locationManagerDidChangeAuthorization:))]
            unsafe fn location_manager_did_change_authorization(
                &self,
                manager: &CLLocationManager,
            ) {
                self.continue_after_authorization(manager);
            }

            #[allow(deprecated)]
            #[unsafe(method(locationManager:didChangeAuthorizationStatus:))]
            unsafe fn location_manager_did_change_authorization_status(
                &self,
                manager: &CLLocationManager,
                _status: CLAuthorizationStatus,
            ) {
                self.continue_after_authorization(manager);
            }
        }
    );

    impl LocationDelegate {
        fn new(
            request_id: u64,
            sender: oneshot::Sender<CurrentLocationResponse>,
            mtm: MainThreadMarker,
        ) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(LocationDelegateIvars {
                requested_location: Cell::new(false),
                completed: Cell::new(false),
                waiters: RefCell::new(vec![(request_id, sender)]),
            });
            unsafe { msg_send![super(this), init] }
        }

        fn add_waiter(&self, request_id: u64, sender: oneshot::Sender<CurrentLocationResponse>) {
            self.ivars().waiters.borrow_mut().push((request_id, sender));
        }

        fn is_completed(&self) -> bool {
            self.ivars().completed.get()
        }

        fn remove_waiter(&self, request_id: u64) -> bool {
            self.ivars()
                .waiters
                .borrow_mut()
                .retain(|(id, _)| *id != request_id);
            !self.ivars().waiters.borrow().is_empty()
        }

        fn continue_after_authorization(&self, manager: &CLLocationManager) {
            let status = unsafe { manager.authorizationStatus() };
            if status == CLAuthorizationStatus::AuthorizedAlways
                || status == CLAuthorizationStatus::AuthorizedWhenInUse
            {
                if !self.ivars().requested_location.replace(true) {
                    unsafe { manager.requestLocation() };
                }
            } else if status == CLAuthorizationStatus::Denied {
                self.finish(
                    manager,
                    CurrentLocationResponse::failure(
                        "permission_denied",
                        "core_location",
                        "Location permission was denied. The user can enable it in System Settings > Privacy & Security > Location Services.",
                    ),
                );
            } else if status == CLAuthorizationStatus::Restricted {
                self.finish(
                    manager,
                    CurrentLocationResponse::failure(
                        "restricted",
                        "core_location",
                        "Location access is restricted by macOS or device policy.",
                    ),
                );
            }
        }

        fn finish(&self, manager: &CLLocationManager, response: CurrentLocationResponse) {
            if self.ivars().completed.replace(true) {
                return;
            }
            unsafe {
                manager.stopUpdatingLocation();
                manager.setDelegate(None);
            }
            for (_, sender) in self.ivars().waiters.borrow_mut().drain(..) {
                let _ = sender.send(response.clone());
            }
        }

        fn cancel(&self, manager: &CLLocationManager) {
            self.ivars().completed.set(true);
            unsafe {
                manager.stopUpdatingLocation();
                manager.setDelegate(None);
            }
            self.ivars().waiters.borrow_mut().clear();
        }
    }

    struct ActiveLocationRequest {
        manager: Retained<CLLocationManager>,
        delegate: Retained<LocationDelegate>,
    }

    thread_local! {
        // CLLocationManager's delegate is weak. Keep both objects alive on the
        // main thread while every caller awaiting this one-shot result settles.
        static ACTIVE_REQUEST: RefCell<Option<ActiveLocationRequest>> = const { RefCell::new(None) };
    }

    fn start_request(request_id: u64, sender: oneshot::Sender<CurrentLocationResponse>) {
        let mut pending_sender = Some(sender);
        let joined_existing = ACTIVE_REQUEST.with_borrow_mut(|active| {
            if active
                .as_ref()
                .is_some_and(|request| request.delegate.is_completed())
            {
                active.take();
            }
            if let Some(request) = active.as_ref() {
                request.delegate.add_waiter(
                    request_id,
                    pending_sender.take().expect("pending location sender"),
                );
                true
            } else {
                false
            }
        });
        if joined_existing {
            return;
        }
        let sender = pending_sender.expect("new location sender");

        let enabled = unsafe { CLLocationManager::locationServicesEnabled_class() };
        if !enabled {
            let _ = sender.send(CurrentLocationResponse::failure(
                "services_disabled",
                "core_location",
                "Location Services are disabled in macOS System Settings.",
            ));
            return;
        }

        let Some(mtm) = MainThreadMarker::new() else {
            let _ = sender.send(CurrentLocationResponse::failure(
                "error",
                "core_location",
                "The native location request did not run on the macOS main thread.",
            ));
            return;
        };
        let manager = unsafe { CLLocationManager::new() };
        let delegate = LocationDelegate::new(request_id, sender, mtm);
        unsafe {
            manager.setDesiredAccuracy(kCLLocationAccuracyHundredMeters);
            manager.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        }

        ACTIVE_REQUEST.with_borrow_mut(|active| {
            *active = Some(ActiveLocationRequest {
                manager: manager.clone(),
                delegate: delegate.clone(),
            });
        });

        let status = unsafe { manager.authorizationStatus() };
        if status == CLAuthorizationStatus::NotDetermined {
            unsafe { manager.requestWhenInUseAuthorization() };
        } else {
            delegate.continue_after_authorization(&manager);
        }
    }

    fn cancel_request(request_id: u64) {
        ACTIVE_REQUEST.with_borrow_mut(|active| {
            let should_remove = active.as_ref().is_some_and(|request| {
                if request.delegate.is_completed() {
                    true
                } else {
                    !request.delegate.remove_waiter(request_id)
                }
            });
            if should_remove {
                if let Some(request) = active.take() {
                    if !request.delegate.is_completed() {
                        request.delegate.cancel(&request.manager);
                    }
                }
            }
        });
    }

    fn clear_completed_request() {
        ACTIVE_REQUEST.with_borrow_mut(|active| {
            if active
                .as_ref()
                .is_some_and(|request| request.delegate.is_completed())
            {
                active.take();
            }
        });
    }

    pub async fn request_current_location(app: tauri::AppHandle) -> CurrentLocationResponse {
        let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        if let Err(error) = app.run_on_main_thread(move || start_request(request_id, sender)) {
            return CurrentLocationResponse::failure(
                "error",
                "core_location",
                format!("Could not start the macOS location request: {error}"),
            );
        }

        match tokio::time::timeout(LOCATION_TIMEOUT, receiver).await {
            Ok(Ok(response)) => {
                let _ = app.run_on_main_thread(clear_completed_request);
                response
            }
            Ok(Err(_)) => CurrentLocationResponse::failure(
                "error",
                "core_location",
                "The macOS location request ended before returning a result.",
            ),
            Err(_) => {
                let _ = app.run_on_main_thread(move || cancel_request(request_id));
                CurrentLocationResponse::failure(
                    "timeout",
                    "core_location",
                    "The macOS location request timed out after 20 seconds.",
                )
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{sample_timestamps, CurrentLocationResponse};

        #[test]
        fn failure_response_never_contains_coordinates() {
            let response = CurrentLocationResponse::failure("timeout", "core_location", "waited");
            assert_eq!(response.status, "timeout");
            assert_eq!(response.latitude, None);
            assert_eq!(response.longitude, None);
            assert_eq!(response.accuracy_m, None);
            assert_eq!(response.observed_at_unix_ms, None);
            assert_eq!(response.sample_age_ms, None);
        }

        #[test]
        fn cached_or_clock_invalid_samples_are_rejected() {
            assert_eq!(sample_timestamps(-301.0, 1_700_000_000.0), None);
            assert_eq!(sample_timestamps(61.0, 1_700_000_000.0), None);
            assert_eq!(sample_timestamps(f64::NAN, 1_700_000_000.0), None);
            assert_eq!(
                sample_timestamps(-5.5, 1_700_000_000.0),
                Some((1_700_000_000_000, 5_500))
            );
        }
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn request_current_location(app: tauri::AppHandle) -> CurrentLocationResponse {
    macos::request_current_location(app).await
}
