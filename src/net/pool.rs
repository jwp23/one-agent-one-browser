use std::sync::{mpsc, Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RequestId(u64);

impl RequestId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
pub struct FetchEvent {
    pub id: RequestId,
    pub url: String,
    pub result: Result<Vec<u8>, String>,
}

pub struct FetchPool {
    job_tx: mpsc::Sender<Job>,
    event_rx: mpsc::Receiver<FetchEvent>,
    next_id: u64,
}

impl FetchPool {
    pub fn new(worker_count: usize) -> FetchPool {
        let worker_count = worker_count.max(1);
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (event_tx, event_rx) = mpsc::channel::<FetchEvent>();
        let shared_rx = Arc::new(Mutex::new(job_rx));

        for _ in 0..worker_count {
            let shared_rx = Arc::clone(&shared_rx);
            let event_tx = event_tx.clone();
            std::thread::spawn(move || worker_loop(shared_rx, event_tx));
        }

        FetchPool {
            job_tx,
            event_rx,
            next_id: 1,
        }
    }

    pub fn fetch_bytes(&mut self, url: String) -> Result<RequestId, String> {
        let id = RequestId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.job_tx
            .send(Job::Fetch { id, url })
            .map_err(|_| "Failed to enqueue fetch: pool is shut down".to_owned())?;
        Ok(id)
    }

    pub fn try_recv(&mut self) -> Option<FetchEvent> {
        self.event_rx.try_recv().ok()
    }
}

enum Job {
    Fetch { id: RequestId, url: String },
}

fn worker_loop(shared_rx: Arc<Mutex<mpsc::Receiver<Job>>>, event_tx: mpsc::Sender<FetchEvent>) {
    loop {
        let job = match shared_rx.lock() {
            Ok(rx) => rx.recv(),
            Err(_) => return,
        };

        let job = match job {
            Ok(job) => job,
            Err(_) => return,
        };

        match job {
            Job::Fetch { id, url } => {
                let result = super::curl::fetch_url_bytes(&url);
                let _ = event_tx.send(FetchEvent { id, url, result });
            }
        }
    }
}

