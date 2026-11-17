//! Sends requests off the UI thread; one at a time.

use std::sync::mpsc::{Receiver, Sender as Channel, channel};
use std::thread;
use std::time::{Duration, Instant};

use application::ports::HttpError;
use application::sending::SendRequest;
use domain::{PreparedRequest, RequestId, Response, Settings};

type Outcome = (u64, RequestId, PreparedRequest, Result<Response, HttpError>);

#[derive(Debug)]
struct InFlight {
    generation: u64,
    request: RequestId,
    started: Instant,
}

pub struct Sender {
    tx: Channel<Outcome>,
    rx: Receiver<Outcome>,
    generation: u64,
    in_flight: Option<InFlight>,
    repaint: egui::Context,
}

impl std::fmt::Debug for Sender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sender")
            .field("in_flight", &self.in_flight)
            .finish_non_exhaustive()
    }
}

impl Sender {
    pub fn new(repaint: egui::Context) -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx,
            generation: 0,
            in_flight: None,
            repaint,
        }
    }

    pub fn start(
        &mut self,
        send: SendRequest,
        request: RequestId,
        prepared: PreparedRequest,
        settings: Settings,
    ) {
        self.generation += 1;
        let generation = self.generation;
        self.in_flight = Some(InFlight {
            generation,
            request,
            started: Instant::now(),
        });
        let tx = self.tx.clone();
        let repaint = self.repaint.clone();
        // Not joined: a cancelled send finishes on its own (within the
        // timeout) and its answer is dropped.
        thread::spawn(move || {
            let result = send.execute(&prepared, &settings);
            // The app may be gone; nobody is left to tell.
            let _ = tx.send((generation, request, prepared, result));
            repaint.request_repaint();
        });
    }

    /// Forgets the request on its way; its answer will be ignored.
    pub fn cancel(&mut self) {
        self.in_flight = None;
    }

    /// How long `request` has been on its way, if it is.
    pub fn sending(&self, request: RequestId) -> Option<Duration> {
        self.in_flight
            .as_ref()
            .filter(|f| f.request == request)
            .map(|f| f.started.elapsed())
    }

    pub fn is_busy(&self) -> bool {
        self.in_flight.is_some()
    }

    /// The answer of the current send, once it arrived.
    pub fn poll(&mut self) -> Option<(RequestId, PreparedRequest, Result<Response, HttpError>)> {
        while let Ok((generation, request, prepared, result)) = self.rx.try_recv() {
            if self
                .in_flight
                .as_ref()
                .is_some_and(|f| f.generation == generation)
            {
                self.in_flight = None;
                return Some((request, prepared, result));
            }
        }
        None
    }
}
