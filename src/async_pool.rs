use core::panic;
use std::collections::VecDeque;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, Condvar, Mutex};
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use crate::error;
use crate::event;
use crate::signal;

type JobFunc = dyn Fn() -> Option<error::Error> + 'static + Send + Sync;
type Job = Box<JobFunc>;

struct PollingJob {
    job: Box<JobFunc>,
    // might not need to be an atomic, since we are passing the whole job object as a arc<mutex>>
    running: AtomicBool,
    waiting: bool,
    timeout: Duration,
    last_t: Instant,
}

impl PollingJob {
    fn new(job: Box<JobFunc>, waiting: bool, timeout: Duration) -> Self {
        Self {
            job,
            running: AtomicBool::new(false),
            waiting,
            timeout,
            last_t: Instant::now(),
        }
    }
}

struct Capacity {
    capacity: usize,
    load: usize,
}

impl Capacity {
    fn new() -> Self {
        Self {
            capacity: 0,
            load: 0,
        }
    }
}

struct AsyncState {
    queue: (Mutex<VecDeque<Message>>, Condvar),
    jobs: Mutex<Vec<Arc<Mutex<PollingJob>>>>,
    capacity: Mutex<Capacity>,
    signal: Arc<Mutex<signal::Signal>>,
}

impl AsyncState {
    fn new(signal: Arc<Mutex<signal::Signal>>) -> Self {
        Self {
            queue: (Mutex::new(VecDeque::new()), Condvar::new()),
            jobs: Mutex::new(Vec::new()),
            capacity: Mutex::new(Capacity::new()),
            signal,
        }
    }
}

// this is stupid since at a given point only one thread can consume the message but let it stay
// here in case i get an idea how to reuse this
// maybe here we can differentiate different types of jobs, interesting idea
enum Message {
    Shutdown,
    NewJob(Job),
    NewPollingJob(Arc<Mutex<PollingJob>>),
}

struct WaitableWorker {
    t: Option<thread::JoinHandle<()>>,
}

impl WaitableWorker {
    fn new(state: Arc<AsyncState>) -> Self {
        let t = thread::spawn(move || {
            {
                let mut capacity = state.capacity.lock().unwrap();
                capacity.capacity = capacity.capacity + 1;
            }
            loop {
                let message: Message;

                {
                    let (lock, cvar) = &state.queue;

                    let mut queue_guard = lock.lock().unwrap();
                    while queue_guard.is_empty() {
                        queue_guard = cvar.wait(queue_guard).unwrap();
                    }

                    message = queue_guard.pop_front().unwrap();
                }

                {
                    let mut capacity = state.capacity.lock().unwrap();
                    capacity.load = capacity.load + 1;
                }

                match message {
                    Message::NewJob(job) => {
                        job();
                    }
                    Message::NewPollingJob(polling_job_mutex) => {
                        let mut job = polling_job_mutex.lock().unwrap();

                        job.last_t = Instant::now();
                        job.running.store(true, Ordering::SeqCst);
                        let r = (job.job)();
                        job.running.store(false, Ordering::SeqCst);

                        if let Some(e) = r {
                            state.signal.lock().unwrap().notify(event::Event::Error(e));
                        }
                    }
                    Message::Shutdown => {
                        break;
                    }
                }

                {
                    let mut capacity = state.capacity.lock().unwrap();
                    capacity.load = capacity.load - 1;
                }
            }

            {
                let mut capacity = state.capacity.lock().unwrap();
                capacity.capacity = capacity.capacity - 1;
                state
                    .signal
                    .lock()
                    .unwrap()
                    .notify(event::Event::Log(String::from(
                        "Shutting down waiting worker",
                    )));
            }
        });

        Self { t: Some(t) }
    }
}

struct PollingWorker {
    t: Option<thread::JoinHandle<()>>,
}

impl PollingWorker {
    fn new(state: Arc<AsyncState>, resolution: Duration) -> Self {
        let thread = thread::spawn(move || {
            state
                .signal
                .lock()
                .unwrap()
                .notify(event::Event::Log(String::from("Starting polling worker")));

            loop {
                //println!("LITERAL START OF LOOP");
                let t = Instant::now();

                if let Ok(mut queue) = state.queue.0.try_lock() {
                    if let Some(Message::Shutdown) = queue.front() {
                        queue.pop_front();
                        break;
                    }
                }

                for job_mutex in state.jobs.lock().unwrap().iter() {
                    let mut should_start;

                    {
                        let job = job_mutex.try_lock();

                        if let Ok(job) = job {
                            let dt = Instant::now() - job.last_t;
                            should_start = dt > job.timeout
                                && (!job.running.load(Ordering::SeqCst) && job.waiting);
                        } else {
                            should_start = false;
                        }
                    }

                    if should_start {
                        let capacity = state.capacity.lock().unwrap();
                        should_start = capacity.load < capacity.capacity;

                        if should_start {
                            let (lock, cvar) = &state.queue;

                            lock.lock()
                                .unwrap()
                                .push_back(Message::NewPollingJob(Arc::clone(job_mutex)));
                            cvar.notify_one();
                        } else {
                            // if we get to here we would need to spawn another thread to handle this,
                            // increase our thread count
                            // time is of the essense here, need to be thought out how to be handled
                            state.signal.lock().unwrap().notify(event::Event::LogError(String::from("This is a big issue, there is no designated worker to take care of this task and possible other scenarios")));
                        }
                    }
                }

                // TODO: handle dt bigger than resolution
                let dt = Instant::now() - t;

                //println!("TIME PASSED FOR TICK {}", dt.as_millis());
                if dt > resolution {
                    state.signal.lock().unwrap().notify(event::Event::LogError(String::from("Got dt biggen than the resolution this is an issue, posslby start another timer/job thread")));
                    panic!("DT BIGGER THAN RESOLUTION");
                }

                let sleep_time = resolution - dt;
                sleep(sleep_time);
            }
        });

        Self { t: Some(thread) }
    }
}

enum Worker {
    Waiting(WaitableWorker),
    Polling(PollingWorker),
}

impl Worker {
    fn join(&mut self) {
        match (self) {
            Worker::Waiting(w) => {
                if let Some(handle) = w.t.take() {
                    handle.join();
                }
            }
            Worker::Polling(w) => {
                if let Some(handle) = w.t.take() {
                    handle.join();
                }
            }
        };
    }
}

pub struct AsyncPool {
    workers: Vec<Worker>,
    polling_attached: AtomicBool,
    polling_resolution: Duration,
    async_state: Arc<AsyncState>,
    signal: Arc<Mutex<signal::Signal>>,
}

impl AsyncPool {
    pub fn new(count: usize, polling_resolution: Duration) -> Self {
        let mut workers = Vec::with_capacity(count);

        let signal = Arc::new(Mutex::new(signal::Signal::new()));

        let async_state = Arc::new(AsyncState::new(Arc::clone(&signal)));

        for _ in 0..count {
            workers.push(Worker::Waiting(WaitableWorker::new(Arc::clone(
                &async_state,
            ))));
        }

        Self {
            workers,
            async_state,
            polling_attached: AtomicBool::new(false),
            polling_resolution,
            signal,
        }
    }

    pub fn submit<F>(&mut self, job: F)
    where
        F: Fn() -> Option<error::Error> + 'static + Send + Sync,
    {
        let (lock, cvar) = &self.async_state.queue;
        lock.lock()
            .unwrap()
            .push_back(Message::NewJob(Box::new(job)));
        cvar.notify_one();
    }

    pub fn attach_job<F>(&mut self, timeout: Duration, job: F)
    where
        F: Fn() -> Option<error::Error> + 'static + Send + Sync,
    {
        let attached_result =
            self.polling_attached
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);

        if let Ok(_) = attached_result {
            self.workers.push(Worker::Polling(PollingWorker::new(
                Arc::clone(&self.async_state),
                self.polling_resolution,
            )));
        }

        let mut jobs = self.async_state.jobs.lock().unwrap();
        jobs.push(Arc::new(Mutex::new(PollingJob::new(
            Box::new(job),
            true,
            timeout,
        ))));
    }

    pub fn connect_listener<F>(&mut self, f: F)
    where
        F: Fn(Arc<event::Event>) + 'static + Send,
    {
        self.signal.lock().unwrap().connect(f);
    }

    pub fn shutdown(&mut self) {
        let mut jobs = self.async_state.jobs.lock().unwrap();
        let mut queue = self.async_state.queue.0.lock().unwrap();

        jobs.clear();
        queue.clear();

        self.workers.iter_mut().for_each(|_| {
            queue.push_back(Message::Shutdown);
        });
    }

    pub fn wait(&mut self) {
        self.workers.iter_mut().for_each(|w| {
            w.join();
        });
    }
}
