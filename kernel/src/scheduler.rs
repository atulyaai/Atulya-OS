use alloc::vec::Vec;
use crate::process::{Process, ProcessState, Context};

pub struct Scheduler {
    processes: Vec<Process>,
    current: Option<usize>,
    tick_count: u64,
    ticks_per_slice: u64,
    switch_pending: bool,
}

static mut SCHEDULER: Option<Scheduler> = None;

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
}

pub fn init() {
    unsafe {
        SCHEDULER = Some(Scheduler::new());
    }
    crate::serial::serial_write_line("Scheduler initialized.");
}

pub fn add_process(proc: Process) -> u32 {
    unsafe { SCHEDULER.as_mut().unwrap().add_process(proc) }
}

pub fn current_pid() -> u32 {
    unsafe { SCHEDULER.as_ref().unwrap().current_pid() }
}

/// Called from timer interrupt. Returns true if context switch is needed.
pub fn tick() -> bool {
    unsafe { SCHEDULER.as_mut().unwrap().tick() }
}

pub fn check_and_schedule() {
    unsafe {
        let s = SCHEDULER.as_mut().unwrap();
        if s.take_switch_pending() {
            s.pick_next();
            crate::serial::serial_write_line("Process switch (cooperative)");
        }
    }
}

/// Get a mutable pointer to the current process's context.
/// Returns null if no current process.
pub fn current_ctx_mut() -> *mut Context {
    unsafe {
        SCHEDULER.as_mut().and_then(|s| {
            s.current.and_then(|i| s.processes.get_mut(i)).map(|p| p.ctx_mut())
        }).unwrap_or(core::ptr::null_mut())
    }
}

/// Pick next process and return pointer for context switching.
pub fn pick_next_and_switch() -> *const Context {
    unsafe { SCHEDULER.as_mut().unwrap().pick_next() }
}

pub fn process_count() -> usize {
    unsafe { SCHEDULER.as_ref().unwrap().process_count() }
}

pub fn list_pids() -> Vec<u32> {
    unsafe { SCHEDULER.as_ref().unwrap().list_pids() }
}
