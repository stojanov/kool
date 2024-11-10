use std::{collections::{HashMap, HashSet}, error::Error, process::{Child, Command}, sync::{atomic::AtomicUsize, mpsc::{self, channel, Receiver, Sender}, Arc, Condvar, Mutex}, time::{Duration, Instant}};

use crate::async_pool::AsyncPool;

type ProcId = usize;

pub struct CommandRunner {
    pool: Arc<Mutex<AsyncPool>>,
    job_result: (Arc<Mutex<Sender<(String, ProcId)>>>, Arc<Mutex<Receiver<(String, ProcId)>>>),
    job_timeout: (Arc<Mutex<Sender<i32>>>, Arc<Mutex<Receiver<i32>>>),
    results: Arc<(Mutex<HashMap<ProcId, String>>, Condvar)>,
    children: Arc<Mutex<HashMap<ProcId, Arc<Mutex<Child>>>>>,
    id: AtomicUsize
}

impl CommandRunner {
    pub fn new(pool: Arc<Mutex<AsyncPool>>) -> Self {
        let (result_tx, result_rx) = mpsc::channel();

        let result_tx = Arc::new(Mutex::new(result_tx));
        let result_rx = Arc::new(Mutex::new(result_rx));

        let (timeout_tx, timeout_rx) = mpsc::channel();

        let timeout_tx = Arc::new(Mutex::new(timeout_tx));
        let timeout_rx = Arc::new(Mutex::new(timeout_rx));

        Self {
            pool,
            job_result: (Arc::clone(&result_tx), Arc::clone(&result_rx)),
            job_timeout: (Arc::clone(&timeout_tx), Arc::clone(&timeout_rx)),
            results: Arc::new((Mutex::new(HashMap::new()), Condvar::new())),
            children: Arc::new(Mutex::new(HashMap::new())),
            id: AtomicUsize::new(0)
        }
    }

    pub fn start(&mut self) {
        let res_rx = Arc::clone(&self.job_result.1);
        let results = Arc::clone(&self.results);

        self.pool.lock().unwrap().attach_job(Duration::from_millis(0),
        move || {
            // here recv will block
            if let Ok(result) = res_rx.lock().unwrap().recv() {
                let (map, cvar) = &*results;
                map.lock().unwrap().insert(result.1, result.0);
                cvar.notify_all();
            }
        });
    }

    fn query_result(&mut self, id: ProcId, timeout: Duration) -> Option<String> {
        let (lock, cvar) = &*self.results;
        let mut map_guard= lock.lock().unwrap();

        let t = Instant::now();
        let key = Arc::new(id);
        let mut wait_time = timeout;

        while !map_guard.contains_key(&key) {
            let (guard, result) = cvar.wait_timeout(map_guard, wait_time).unwrap();
            map_guard = guard;

            if !result.timed_out() {
                if let Some(resolved) = map_guard.get(&key) {
                    let rtn = resolved.clone();
                    map_guard.remove(&key);
                    return Some(rtn);
                }
            }

            let dt = Instant::now() - t;
            wait_time = wait_time - dt;

            if dt >= timeout || wait_time == Duration::from_millis(0)
            {
                break;
            }
        }

        None
    }

    pub fn run(&mut self, path: String, args: Vec<String>, timeout: Duration) -> Option<String> {
        let children = Arc::clone(&self.children);

        let id = self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        self.pool.lock().unwrap().submit(
            move || {
                let res= Command::new(&path).args(&args).spawn();

                if let Ok(child) = res {

                    let child = Arc::new(Mutex::new(child));

                    children.lock().unwrap().insert(id, Arc::clone(&child));

                    let mut child = child.lock().unwrap();
                    if let Ok(status) = child.wait_with_output(){

                    }
                }


                // TODO signal error
            }
        );

        let result = self.query_result(id, timeout);
    }
}
