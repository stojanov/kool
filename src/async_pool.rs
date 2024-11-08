use std::thread;
use std::sync::{ Arc, Condvar, Mutex, atomic::AtomicBool, atomic::Ordering };
use std::collections::VecDeque;
use std::time::{ Duration, Instant };

type JobFunc = dyn FnOnce() + 'static + Send;
type Job = Arc<JobFunc>;

struct PollingJob {
    job: Job, 
    running: AtomicBool,
    waiting: bool,
    timeout: Duration,
    last_t: Instant
}

impl PollingJob {
    fn new(job: Job, running: AtomicBool, waiting: bool, timeout: Duration, last_t: Instant) -> Self {
        Self { job, running, waiting, timeout, last_t }
    }
}

// this is stupid since at a given point only one thread can consume the message but let it stay
// here in case i get an idea how to reuse this
// maybe here we can differentiate different types of jobs, interesting idea
enum Message {
    Shutdown,
    NewJob(Job)
} 

struct WaitableWorker {
    t: thread::JoinHandle<()>, 
}

impl WaitableWorker {
    fn new(queue: Arc<(Mutex<VecDeque<Message>>, Condvar)>)-> Self {
        let t = thread::spawn(move || {
            loop {
                let message: Message;

                {
                    let (lock, cvar) = &*queue;

                    let mut queue_guard= lock.lock().unwrap();
                    while queue_guard.is_empty() {
                        queue_guard = cvar.wait(queue_guard).unwrap();
                    }

                    message = queue_guard.pop_front().unwrap();
                }

                match message {
                    Message::NewJob(job) => {
                        job();
                    },
                    Message::Shutdown => {
                        break;
                    }
                }
            }
        });

        Self {
            t
        }
    }
}

struct PollingWorker {
    t: thread::JoinHandle<()>,
}

impl PollingWorker {
    fn new(queue: Arc<(Mutex<VecDeque<Message>>, Condvar)>, jobs: Arc<Mutex<Vec<PollingJob>>>) -> Self {
        let thread = thread::spawn(move || {
            let mut t = Instant::now();

            loop {
                for job in jobs.lock().unwrap().iter() {
                    let dt = Instant::now() - job.last_t;

                    if dt > job.timeout && (job.waiting && !job.running.load(Ordering::SeqCst)) {
                        let (lock, cvar) = &*queue;
                        lock.lock().unwrap().push_back(Message::NewJob(Box::clone(job.job)));
                        cvar.notify_one();
                    }
                }
            }
        });

        Self {
            t: thread
        }
    }
}

enum Worker {
    Waiting(WaitableWorker),
    Polling(PollingWorker)
}

pub struct AsyncPool { 
    workers: Vec<Worker>,
    queue: Arc<(Mutex<VecDeque<Message>>, Condvar)>,
    jobs: Arc<Mutex<Vec<PollingJob>>>,
    polling_attached: AtomicBool
}

impl AsyncPool {
    pub fn new(count: usize) -> Self {
        let mut workers= Vec::with_capacity(count);

        let queue = Arc::new((
            Mutex::new(VecDeque::new()),
            Condvar::new()
        ));

        let jobs = Arc::new(Mutex::new(Vec::new()));

        for _ in 0..count  {
            workers.push(Worker::Waiting(WaitableWorker::new(Arc::clone(&queue))));
        }

        Self {
            workers,
            queue,
            jobs,
            polling_attached: AtomicBool::new(false)
        }
    }

    fn submit(&mut self, job: Job) {
        let (lock, cvar) = &*self.queue;
        lock.lock().unwrap().push_back(Message::NewJob(job));
        cvar.notify_one();
    } 

    pub fn attach_job(&mut self, job: &JobFunc, dur: Duration) {
        let attached_result= self.polling_attached.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);

        if let Ok(_) = attached_result {
            self.workers.push(Worker::Polling(PollingWorker::new(
                        Arc::clone(&self.queue),
                        Arc::clone(&self.jobs))));
        }

        let jobs = self.jobs.lock().unwrap();

        jobs.push(PollingJob {
            job: Arc::new(job),
            running: AtomicBool::new(false),
            waiting: true,
            timeout,
            last_t: Instant::now()
        });
    }
}


