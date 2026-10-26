//! Yhden vastauksen taustatyöt: keskeneräinen ja katkennut työ erotetaan.
use std::sync::mpsc::{Receiver, RecvError, TryRecvError};

pub(super) fn poll<T>(receiver: &Receiver<T>) -> Option<Result<T, RecvError>> {
    match receiver.try_recv() {
        Ok(value) => Some(Ok(value)),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => Some(Err(RecvError)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, thread};

    #[test]
    fn pending_worker_is_not_reported_as_failed() {
        let (_sender, receiver) = mpsc::channel::<()>();
        assert!(poll(&receiver).is_none());
    }

    #[test]
    fn worker_exiting_without_response_is_not_left_pending() {
        let (sender, receiver) = mpsc::channel::<()>();
        thread::spawn(move || drop(sender)).join().unwrap();
        assert!(matches!(poll(&receiver), Some(Err(_))));
    }

    #[test]
    fn panicked_worker_is_reported_as_stopped() {
        let (sender, receiver) = mpsc::channel::<()>();
        let worker = thread::spawn(move || {
            let _sender = sender;
            panic!("Simuloitu taustatyön keskeytyminen");
        });
        assert!(worker.join().is_err());
        assert!(matches!(poll(&receiver), Some(Err(_))));
    }

    #[test]
    fn queued_response_survives_worker_exit() {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || sender.send(Ok::<_, String>(42)).unwrap())
            .join()
            .unwrap();
        assert_eq!(poll(&receiver), Some(Ok(Ok(42))));
    }

    #[test]
    fn operation_error_survives_worker_exit() {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || sender.send(Err::<(), _>("Palvelin ei vastaa")).unwrap())
            .join()
            .unwrap();
        assert_eq!(poll(&receiver), Some(Ok(Err("Palvelin ei vastaa"))));
    }
}
