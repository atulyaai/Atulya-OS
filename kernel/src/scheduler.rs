use alloc::vec::Vec;
use crate::process::{Process, ProcessState, Context};

pub struct Scheduler {
    processes: Vec<Process>,
    current: Option<usize>,
    tick_count: u64,
    ticks_per_slice: u64,
    switch_pending: bool,
}

static SCHEDULER: spin::Mutex<Option<Scheduler>> = spin::Mutex::new(None);

impl Scheduler {
    fn new() -> Self {
        let mut procs = Vec::new();
        let idle = Process::new_idle();
        procs.push(idle);

        Scheduler {
            processes: procs,
            current: Some(0),
            tick_count: 0,
            ticks_per_slice: 5, // switch every 5 ticks (~280ms at 18Hz)
            switch_pending: false,
        }
    }

    pub fn add_process(&mut self, proc: Process) -> u32 {
        let pid = proc.pid();
        self.processes.push(proc);
        pid
    }

    pub fn current_pid(&self) -> u32 {
        self.current
            .and_then(|i| self.processes.get(i))
            .map(|p| p.pid())
            .unwrap_or(0)
    }

    /// Called on every timer tick. Returns true if a context switch should happen.
    pub fn tick(&mut self) -> bool {
        self.tick_count += 1;
        if self.tick_count >= self.ticks_per_slice {
            self.tick_count = 0;
            return true;
        }
        false
    }

    pub fn take_switch_pending(&mut self) -> bool {
        if self.switch_pending {
            self.switch_pending = false;
            true
        } else {
            false
        }
    }

    /// Pick the next process to run. Returns a pointer to its Context.
    /// The caller must ensure the returned pointer remains valid (i.e. the
    /// process is not dropped while it's scheduled).
    pub fn pick_next(&mut self) -> *const Context {
        let current_idx = match self.current {
            Some(i) => i,
            None => return core::ptr::null(),
        };

        // Demote current process
        if let Some(proc) = self.processes.get_mut(current_idx) {
            if proc.state() == ProcessState::Running {
                proc.set_state(ProcessState::Ready);
            }
        }

        // Round-robin: find next Ready process that is NOT the current one
        let len = self.processes.len();
        let mut next_idx = (current_idx + 1) % len;
        let mut attempts = 0;

        while attempts < len {
            if let Some(proc) = self.processes.get(next_idx) {
                if proc.state() == ProcessState::Ready && next_idx != current_idx {
                    break;
                }
            }
            next_idx = (next_idx + 1) % len;
            attempts += 1;
        }

        if attempts < len {
            if let Some(proc) = self.processes.get_mut(next_idx) {
                proc.set_state(ProcessState::Running);
            }
            self.current = Some(next_idx);
            self.processes[next_idx].ctx() as *const Context
        } else {
            // No other process ready — resume current
            if let Some(proc) = self.processes.get_mut(current_idx) {
                proc.set_state(ProcessState::Running);
            }
            core::ptr::null()
        }
    }

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    pub fn list_pids(&self) -> Vec<u32> {
        self.processes.iter().map(|p| p.pid()).collect()
    }

    pub fn list_processes(&self) -> Vec<(u32, &'static str, &'static str)> {
        self.processes.iter().map(|p| {
            let state_str = match p.state() {
                ProcessState::Running => "Running",
                ProcessState::Ready => "Ready",
                ProcessState::Blocked => "Blocked",
                ProcessState::Dead => "Dead",
            };
            (p.pid(), p.name(), state_str)
        }).collect()
    }

    pub fn kill_process(&mut self, pid: u32) -> bool {
        if let Some(proc) = self.processes.iter_mut().find(|p| p.pid() == pid) {
            proc.set_state(ProcessState::Dead);
            true
        } else {
            false
        }
    }
}

pub fn init() {
    *SCHEDULER.lock() = Some(Scheduler::new());
    crate::serial::serial_write_line("Scheduler initialized.");
}

pub fn add_process(proc: Process) -> u32 {
    SCHEDULER.lock().as_mut().unwrap().add_process(proc)
}

pub fn spawn_kernel_thread(name: &'static str, entry: extern "C" fn()) -> u32 {
    let proc = Process::new_kernel_thread(name, entry, 1);
    add_process(proc)
}

pub fn current_pid() -> u32 {
    SCHEDULER.lock().as_ref().unwrap().current_pid()
}

pub fn list_processes() -> Vec<(u32, &'static str, &'static str)> {
    SCHEDULER.lock().as_ref().map(|s| s.list_processes()).unwrap_or_default()
}

pub fn kill_process(pid: u32) -> bool {
    SCHEDULER.lock().as_mut().map(|s| s.kill_process(pid)).unwrap_or(false)
}

/// Called from timer interrupt. Returns true if context switch is needed.
pub fn tick() -> bool {
    SCHEDULER.lock().as_mut().unwrap().tick()
}

pub fn check_and_schedule() {
    let mut guard = SCHEDULER.lock();
    let s = guard.as_mut().unwrap();
    if s.take_switch_pending() {
        s.pick_next();
        crate::serial::serial_write_line("Process switch (cooperative)");
    }
}

/// Get a mutable pointer to the current process's context.
/// Returns null if no current process.
pub fn current_ctx_mut() -> *mut Context {
    SCHEDULER.lock().as_mut().and_then(|s| {
        s.current.and_then(|i| s.processes.get_mut(i)).map(|p| p.ctx_mut())
    }).unwrap_or(core::ptr::null_mut())
}

/// Pick next process and return pointer for context switching.
pub fn pick_next_and_switch() -> *const Context {
    SCHEDULER.lock().as_mut().unwrap().pick_next()
}

pub fn process_count() -> usize {
    SCHEDULER.lock().as_ref().unwrap().process_count()
}

pub fn list_pids() -> Vec<u32> {
    SCHEDULER.lock().as_ref().unwrap().list_pids()
}
